using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;
using System.Security;

namespace Ngs.Interop.Marshaller
{
	[SecuritySafeCriticalAttribute]
	public delegate IntPtr[] CcwFactory();

	public struct CcwFactoryInfo
	{
		/** `TypeInfo` that represents a type containing (static) thunk methods. */
		public TypeInfo ClassTypeInfo { get; set; }
		public MethodInfo FactoryMethodInfo { get; set; }
		public CcwFactory FactoryDelegate { get; set; }
	}

	public class CcwGenerator
	{
		ModuleBuilder moduleBuilder;
		Dictionary<Type, CcwFactoryInfo> ccws = new Dictionary<Type, CcwFactoryInfo>();

		public CcwGenerator(ModuleBuilder moduleBuilder)
		{
			this.moduleBuilder = moduleBuilder;
		}

		public CcwFactoryInfo CreateCcwFactory(Type interfaceType)
		{
			CcwFactoryInfo ccw;
			if (!ccws.TryGetValue(interfaceType, out ccw))
			{
				ccw = new CcwFactoryInfo();
				ccw.ClassTypeInfo = CreateCcwClass(new InterfaceInfo(interfaceType));
				ccw.FactoryMethodInfo = ccw.ClassTypeInfo.GetDeclaredMethod("Create");
				ccw.FactoryDelegate = (CcwFactory)
					ccw.FactoryMethodInfo.CreateDelegate(typeof(CcwFactory));
				ccws[interfaceType] = ccw;
			}
			return ccw;
		}

		TypeInfo CreateCcwClass(InterfaceInfo interfaceInfo)
		{
			var typeBuilder = moduleBuilder.DefineType(interfaceInfo.Type.FullName + "<CCW>",
			                                           TypeAttributes.Class | TypeAttributes.Sealed |
			                                           TypeAttributes.NotPublic);
			
			DefineCcwClass(interfaceInfo, typeBuilder);
			return typeBuilder.CreateTypeInfo();
		}

		static void DefineCcwClass(InterfaceInfo interfaceInfo, TypeBuilder typeBuilder)
		{
			var type = typeBuilder.AsType();

			var uniqifier = new Utils.UniqueNameGenerator();
			uniqifier.Uniquify("Create"); // reserve

			var methods = new List<CcwMethodInfo>();
			foreach (var method in interfaceInfo.ComMethodInfos)
			{
				methods.Add(CreateCcwMethod(method, typeBuilder, uniqifier));
			}

			CreateCcwFactoryMethod(typeBuilder, methods.ToArray(), uniqifier);
		}

		struct CcwMethodInfo
		{
			public MethodBuilder methodBuilder;
			public Type[] parameterTypes;
		}

		static readonly CustomAttributeBuilder securityCriticalAttributeCab = 
			new CustomAttributeBuilder(typeof(SecurityCriticalAttribute).GetTypeInfo().DeclaredConstructors.First((ctor) => ctor.GetParameters().Length == 0),
				new object[] {});

		static MethodBuilder CreateCcwFactoryMethod(TypeBuilder typeBuilder, CcwMethodInfo[] vtable, Utils.UniqueNameGenerator uniqifier)
		{
			var nativeObjectType = typeof(INativeObject<>).GetTypeInfo().MakeGenericType(typeBuilder.AsType());
			var methodBuilder = typeBuilder.DefineMethod("Create",
			                                             MethodAttributes.Static | MethodAttributes.Public | MethodAttributes.HideBySig,
														 CallingConventions.Standard,
														 typeof(IntPtr[]), new Type[] { });

			methodBuilder.SetCustomAttribute(securityCriticalAttributeCab);

			var gen = methodBuilder.GetILGenerator();
			gen.Emit(OpCodes.Ldc_I4, vtable.Length);
			gen.Emit(OpCodes.Newarr, typeof(IntPtr));
			for (int i = 0; i < vtable.Length; ++i)
			{
				var ccwMethodInfo = vtable[i];
				
				gen.Emit(OpCodes.Dup);
				gen.Emit(OpCodes.Ldc_I4, i);
				gen.Emit(OpCodes.Ldftn, ccwMethodInfo.methodBuilder);
				gen.Emit(OpCodes.Stelem, typeof(IntPtr));

				// add MonoPInvokeCallbackAttribute so this method is included in
				// full AOT
				/*var mpca = new CustomAttributeBuilder(typeof(MonoPInvokeCallbackAttribute)
					.GetTypeInfo()
					.DeclaredConstructors.First((ctor2) => ctor2.GetParameters().Length == 1),
						new object[] { delegateType });
				ccwMethodInfo.methodBuilder.SetCustomAttribute(mpca);

				delegateType.CreateTypeInfo();*/
			}

			gen.Emit(OpCodes.Ret);

			return methodBuilder;
		}

		static CcwMethodInfo CreateCcwMethod(ComMethodInfo comMethodInfo, TypeBuilder typeBuilder,
									Utils.UniqueNameGenerator methodNameUniqifier)
		{
			var methodInfo = comMethodInfo.MethodInfo;
			var methodNameTemplate = methodInfo.Name;
			var methodName = methodNameUniqifier.Uniquify(methodNameTemplate);
			var paramTypes = new List<Type>();
			paramTypes.Add(typeof(IntPtr));
			paramTypes.AddRange(comMethodInfo.ParameterInfos.Select((p) => p.IsOut ?
			 	p.ValueMarshaller.NativeParameterType.MakePointerType() : p.ValueMarshaller.NativeParameterType));
			var methodBuilder = typeBuilder.DefineMethod(methodName,
														 MethodAttributes.HideBySig | MethodAttributes.Static, CallingConventions.Standard,
														 comMethodInfo.NativeReturnType,
														 paramTypes.ToArray());

			methodBuilder.DefineParameter(1, ParameterAttributes.In,
				"this");
				
			// define a method
			foreach (var parameter in methodInfo.GetParameters())
			{
				methodBuilder.DefineParameter(parameter.Position + 2,
					parameter.Attributes, parameter.Name);
			}

			CreateCcwMethodBody(comMethodInfo, methodBuilder);

			return new CcwMethodInfo() {
				methodBuilder = methodBuilder,
				parameterTypes = paramTypes.ToArray()	
			};
		}

		static readonly MethodInfo targetGetter = typeof(GCHandle).GetTypeInfo()
			.GetDeclaredProperty(nameof(GCHandle.Target)).GetMethod;

		static readonly MethodInfo hresultGetter = typeof(Exception).GetTypeInfo()
			.GetDeclaredProperty(nameof(Exception.HResult)).GetMethod;

		static void CreateCcwMethodBody(ComMethodInfo comMethodInfo, MethodBuilder methodBuilder)
		{
			var methodInfo = comMethodInfo.MethodInfo;
			var gen = methodBuilder.GetILGenerator();

			var returnTypeNative = comMethodInfo.ReturnValueMarshaller?.NativeParameterType.MakePointerType()
				?? comMethodInfo.NativeReturnType;
			LocalBuilder returnValueNativeLocal = returnTypeNative != typeof(void) ?
				gen.DeclareLocal(returnTypeNative) : null;
			
			// create parameter marshallers
			var paramInfos = comMethodInfo.ParameterInfos
				.Select((p) =>
				{
					var marshaller = p.ValueMarshaller;
					var runtimeLocal = gen.DeclareLocal(p.Type);
					var toNativeGenerator = p.IsOut ? marshaller.CreateToNativeGenerator(gen) : null;
					var toRuntimeGenerator = p.IsIn ? marshaller.CreateToRuntimeGenerator(gen) : null;
					var nativeType = p.IsOut ? marshaller.NativeParameterType.MakePointerType() : marshaller.NativeParameterType;
					Storage nativeStorage;

					if (p.IsReturnValue)
					{
						nativeStorage = new ParameterStorage(gen, nativeType, comMethodInfo.ParameterInfos.Count());
						nativeStorage = new IndirectStorage(nativeStorage);
					}
					else if (p.IsOut)
					{
						nativeStorage = new ParameterStorage(gen, p.ParameterInfo.ParameterType, p.ParameterInfo.Position + 1);
						nativeStorage = new IndirectStorage(nativeStorage);
					}
					else
					{
						nativeStorage = new ParameterStorage(gen, p.Type, p.ParameterInfo.Position + 1);
					}

					return new
					{
						ParameterInfo = p,
						RuntimeLocal = runtimeLocal,
						Marshaller = marshaller,
						ToNativeGenerator = toNativeGenerator,
						ToRuntimeGenerator = toRuntimeGenerator,
						RuntimeStorage = new LocalStorage(gen, runtimeLocal),
						NativeStorage = nativeStorage
					};
				}).ToArray();

			// start try/catch
			if (comMethodInfo.ReturnsHresult) {
				gen.BeginExceptionBlock();
			}

			// marshal in parameters
			foreach (var paramInfo in paramInfos)
			{
				paramInfo.ToRuntimeGenerator?.EmitToRuntime(paramInfo.NativeStorage, paramInfo.RuntimeStorage,
					move: false);
			}

			// retrieve GCHandle (read ComClassHeader.handle)
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Sizeof, typeof(IntPtr)); // skip vtable
			gen.Emit(OpCodes.Add);

			// GCHandle.Target
			gen.Emit(OpCodes.Call, targetGetter);

			// convert to the target interface type
			gen.Emit(OpCodes.Castclass, comMethodInfo.MethodInfo.DeclaringType);

			// push each parameters
			foreach (var paramInfo in paramInfos)
			{
				if (paramInfo.ParameterInfo.IsReturnValue)
				{
					continue;
				}
				if (paramInfo.ParameterInfo.IsOut)
				{
					// out/inout parameter
					gen.Emit(OpCodes.Ldloca, paramInfo.RuntimeLocal);
				}
				else
				{
					paramInfo.RuntimeStorage.EmitLoad();
				}
			}

			// call it!
			gen.Emit(OpCodes.Callvirt, comMethodInfo.MethodInfo);

			if (comMethodInfo.ReturnValueMarshaller != null)
			{
				if (comMethodInfo.ReturnsHresult)
				{
					throw new InvalidOperationException();
				}

				// marshal real return value
				var runtimeLocal = gen.DeclareLocal(comMethodInfo.MethodInfo.ReturnType);
				gen.Emit(OpCodes.Stloc, runtimeLocal);

				comMethodInfo.ReturnValueMarshaller.CreateToNativeGenerator(gen)
							 .EmitToNative(new LocalStorage(gen, runtimeLocal),
										   new LocalStorage(gen, returnValueNativeLocal));
			}
			foreach (var paramInfo in paramInfos)
			{
				// save [out, retval]
				if (paramInfo.ParameterInfo.IsReturnValue)
				{
					paramInfo.RuntimeStorage.EmitStore();
				}
			}

			foreach (var paramInfo in paramInfos)
			{
				// marshal "out" parameters and optionally return value parameter
				if (paramInfo.ToNativeGenerator != null)
				{
					// drop existing values
					paramInfo.ToNativeGenerator.EmitDestructNativeValue(paramInfo.NativeStorage);

					// marshal
					paramInfo.ToNativeGenerator.EmitToNative(paramInfo.RuntimeStorage, paramInfo.NativeStorage);
				}
			}

			// handle exceptions
			if (comMethodInfo.ReturnsHresult)
			{
				gen.Emit(OpCodes.Ldc_I4_0);
				gen.Emit(OpCodes.Stloc, returnValueNativeLocal);

				gen.BeginCatchBlock(typeof(Exception));

				// get hresult
				gen.Emit(OpCodes.Callvirt, hresultGetter);

				// store HRESULT
				gen.Emit(OpCodes.Stloc, returnValueNativeLocal);

				gen.EndExceptionBlock();
			}

			// return
			if (returnValueNativeLocal != null)
			{
				gen.Emit(OpCodes.Ldloc, returnValueNativeLocal);
			}
			gen.Emit(OpCodes.Ret);
		}
		
	}
}
