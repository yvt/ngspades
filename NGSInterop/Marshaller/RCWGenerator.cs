using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
	public delegate INativeObject<T> RcwFactory<out T>(IntPtr interfacePtr) where T : class, IUnknown;

	struct RcwFactoryInfo<T> where T : class, IUnknown
	{
		public TypeInfo ClassTypeInfo { get; set; }
		public MethodInfo FactoryMethodInfo { get; set; }
		public RcwFactory<T> FactoryDelegate { get; set; }
	}

	sealed class RcwGenerator
	{
		static readonly MethodInfo throwExceptionForHRMethod =
			typeof(Marshal).GetRuntimeMethod(nameof(Marshal.ThrowExceptionForHR), new Type[] { typeof(int) });

		ModuleBuilder moduleBuilder;
		Dictionary<Type, RcwFactoryInfo<IUnknown>> rcws = new Dictionary<Type, RcwFactoryInfo<IUnknown>>();

		public RcwGenerator(ModuleBuilder moduleBuilder)
		{
			this.moduleBuilder = moduleBuilder;
		}

		public RcwFactoryInfo<IUnknown> CreateRCWFactory(Type interfaceType)
		{
			RcwFactoryInfo<IUnknown> rcw;

			if (!rcws.TryGetValue(interfaceType, out rcw))
			{

			}
			throw new NotImplementedException();
		}

		public RcwFactoryInfo<T> CreateRCWFactory<T>() where T : class, IUnknown
		{
			var info = CreateRCWFactory(typeof(T));
			return new RcwFactoryInfo<T>()
			{
				ClassTypeInfo = info.ClassTypeInfo,
				FactoryMethodInfo = info.FactoryMethodInfo,
				FactoryDelegate = (RcwFactory<T>) info.FactoryDelegate
			};
		}

		public TypeInfo CreateRCWClass(InterfaceInfo interfaceInfo)
		{
			var typeBuilder = moduleBuilder.DefineType(interfaceInfo.Type.FullName + "<RCW>",
			                                           TypeAttributes.Class | TypeAttributes.Sealed |
			                                           TypeAttributes.NotPublic);
			
			CreateRCWClass(interfaceInfo, typeBuilder);
			return typeBuilder.CreateTypeInfo();
		}

		static readonly MethodInfo nativeInterfacePtrGetter = typeof(INativeObject<>)
			.GetRuntimeProperty(nameof(INativeObject<IUnknown>.NativeInterfacePtr)).GetMethod;
		static readonly MethodInfo nativeIUnknownPtrGetter = typeof(INativeObject<>)
			.GetRuntimeProperty(nameof(INativeObject<IUnknown>.NativeIUnknownPtr)).GetMethod;
		static readonly MethodInfo interfaceGetter = typeof(INativeObject<>)
			.GetRuntimeProperty(nameof(INativeObject<IUnknown>.Interface)).GetMethod;

		static void CreateRCWClass(InterfaceInfo interfaceInfo, TypeBuilder typeBuilder)
		{
			var type = typeBuilder.AsType();

			typeBuilder.AddInterfaceImplementation(typeof(INativeObject<>).MakeGenericType(type));

			foreach (var theInterface in interfaceInfo.AllImplementedInterfaces)
			{
				typeBuilder.AddInterfaceImplementation(theInterface);
			}

			FieldInfo interfacePtrField = typeBuilder.DefineField("interfacePtr", typeof(IntPtr), 0);
			FieldInfo unknownPtrField = typeBuilder.DefineField("iUnknownPtr", typeof(IntPtr), 0);

			// generate constructor
			var ctor = CreateRCWConstructor(typeBuilder, interfacePtrField, unknownPtrField);

			// generate finalizer
			CreateRCWFinalizer(typeBuilder);

			// generate factory
			CreateRCWFactoryMethod(typeBuilder, ctor);

			var methodNameUniqifier = new Utils.UniqueNameGenerator();
			methodNameUniqifier.Uniquify("interfacePtr");
			methodNameUniqifier.Uniquify("iUnknownPtr");
			methodNameUniqifier.Uniquify("Finalize");
			methodNameUniqifier.Uniquify("Create");
			foreach (var method in interfaceInfo.ComMethodInfos)
			{
				CreateRCWMethod(method, typeBuilder, interfacePtrField, methodNameUniqifier);
			}

			// implement INativeObject
			ImplementFieldGetter(typeBuilder, interfacePtrField,
								 nativeInterfacePtrGetter.MakeGenericMethod(type), methodNameUniqifier);
			ImplementFieldGetter(typeBuilder, unknownPtrField,
			                     nativeIUnknownPtrGetter.MakeGenericMethod(type), methodNameUniqifier);
			ImplementInterfaceProperty(typeBuilder, interfaceGetter.MakeGenericMethod(type), methodNameUniqifier);
		}

		static readonly MethodInfo comGuidGetter = typeof(InterfaceRuntimeInfo<IUnknown>)
			.GetRuntimeProperty(nameof(InterfaceRuntimeInfo<IUnknown>.ComGuid)).GetMethod;
		static readonly MethodInfo addRefMethod = typeof(IUnknown)
			.GetRuntimeMethod(nameof(IUnknown.AddRef), new Type[] { });
		static readonly MethodInfo releaseMethod = typeof(IUnknown)
			.GetRuntimeMethod(nameof(IUnknown.Release), new Type[] { });
		static readonly MethodInfo queryNativeInterfaceMethod = typeof(IUnknown)
			.GetRuntimeMethod(nameof(IUnknown.QueryNativeInterface), new Type[] { typeof(Guid) });
		
		static ConstructorBuilder CreateRCWConstructor(TypeBuilder typeBuilder, FieldInfo interfacePtrField, FieldInfo unknownPtrField)
		{
			var ctorBuilder = typeBuilder.DefineConstructor(MethodAttributes.Public, CallingConventions.Standard,
			                                                new Type[] { typeof(IntPtr) });
			ctorBuilder.DefineParameter(1, ParameterAttributes.In, "interfacePtr");

			var gen = ctorBuilder.GetILGenerator();

			// initialize this.interfacePtr
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ldarg_1);
			gen.Emit(OpCodes.Stfld, interfacePtrField);

			// AddRef
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Callvirt, addRefMethod);
			gen.Emit(OpCodes.Pop);

			// initialize this.iUnknownPtr
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Call, comGuidGetter); // get IUnknown guid
			gen.Emit(OpCodes.Callvirt, queryNativeInterfaceMethod);
			gen.Emit(OpCodes.Stfld, unknownPtrField);

			return ctorBuilder;
		}

		static MethodBuilder CreateRCWFactoryMethod(TypeBuilder typeBuilder, ConstructorBuilder ctorInfo)
		{
			var methodBuilder = typeBuilder.DefineMethod("Create",
			                                             MethodAttributes.Static | MethodAttributes.Public,
														 CallingConventions.Standard,
														 typeof(void), new Type[] { });

			var gen = methodBuilder.GetILGenerator();
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Newobj, ctorInfo);

			return methodBuilder;
		}

		static void CreateRCWFinalizer(TypeBuilder typeBuilder)
		{
			var methodBuilder = typeBuilder.DefineMethod("Finalize",
			                                             MethodAttributes.Virtual,
														 CallingConventions.Standard,
														 typeof(void), new Type[] { });

			var gen = methodBuilder.GetILGenerator();

			gen.Emit(OpCodes.Ldarg_0);
			gen.EmitCall(OpCodes.Callvirt, releaseMethod, null);
			gen.Emit(OpCodes.Pop);
		}

		static void ImplementInterfaceProperty(TypeBuilder typeBuilder, MethodInfo getterMethodInfo,
										 Utils.UniqueNameGenerator methodNameUniqifier)
		{
			var methodNameTemplate = getterMethodInfo.DeclaringType.Name + "." + getterMethodInfo.Name;
			var methodName = methodNameUniqifier.Uniquify(methodNameTemplate);
			var methodBuilder = typeBuilder.DefineMethod(methodName,
														 MethodAttributes.Public, CallingConventions.Standard,
														 getterMethodInfo.ReturnType,
														 new Type[] { });
			typeBuilder.DefineMethodOverride(methodBuilder, getterMethodInfo);

			var gen = methodBuilder.GetILGenerator();
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ret);
		}

		static void ImplementFieldGetter(TypeBuilder typeBuilder, FieldInfo fieldInfo, MethodInfo getterMethodInfo, 
		                                 Utils.UniqueNameGenerator methodNameUniqifier)
		{
			var methodNameTemplate = getterMethodInfo.DeclaringType.Name + "." + getterMethodInfo.Name;
			var methodName = methodNameUniqifier.Uniquify(methodNameTemplate);
			var methodBuilder = typeBuilder.DefineMethod(methodName,
				MethodAttributes.Public);
			typeBuilder.DefineMethodOverride(methodBuilder, getterMethodInfo);
			methodBuilder.SetReturnType(getterMethodInfo.ReturnType);

			var gen = methodBuilder.GetILGenerator();
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ldfld, fieldInfo);
			gen.Emit(OpCodes.Ret);
		}

		static void CreateRCWMethod(ComMethodInfo comMethodInfo, TypeBuilder typeBuilder,
									FieldInfo ptrFieldInfo, Utils.UniqueNameGenerator methodNameUniqifier)
		{
			var methodInfo = comMethodInfo.MethodInfo;
			var methodNameTemplate = comMethodInfo.MethodInfo.DeclaringType.Name + "." + methodInfo.Name;
			var methodName = methodNameUniqifier.Uniquify(methodNameTemplate);
			var methodBuilder = typeBuilder.DefineMethod(methodName,
														 MethodAttributes.Public, CallingConventions.Standard,
														 methodInfo.ReturnType,
														 methodInfo.GetParameters().Select((p) => p.ParameterType).ToArray());
			typeBuilder.DefineMethodOverride(methodBuilder, methodInfo);

			// define a method

			int index = 0;
			foreach (var parameter in methodInfo.GetParameters())
			{
				methodBuilder.DefineParameter(index,
					parameter.Attributes, parameter.Name);
				++index;
			}

			// generate a method body
			var gen = methodBuilder.GetILGenerator();

			LocalBuilder returnValueLocal = methodInfo.ReturnType != typeof(void) ?
															   gen.DeclareLocal(methodInfo.ReturnType) : null;

			// create parameter marshallers
			var paramInfos = comMethodInfo.ParameterInfos
				.Select((p) =>
				{
					var marshaller = p.ValueMarshaller;
					var nativeLocal = gen.DeclareLocal(marshaller.NativeParameterType);
					var toNativeGenerator = p.IsIn ? marshaller.CreateToNativeGenerator(gen) : null;
					var toRuntimeGenerator = p.IsOut ? marshaller.CreateToRuntimeGenerator(gen) : null;
					Storage storage;

					if (p.IsReturnValue)
					{
						storage = new LocalStorage(gen, returnValueLocal);
					}
					else if (p.IsOut)
					{
						storage = new ParameterStorage(gen, p.Type.MakePointerType(), p.ParameterInfo.Position);
						storage = new IndirectStorage(storage);
					}
					else
					{
						storage = new ParameterStorage(gen, p.Type, p.ParameterInfo.Position);
					}

					return new
					{
						ParameterInfo = p,
						NativeLocal = nativeLocal,
						Marshaller = marshaller,
						ToNativeGenerator = toNativeGenerator,
						ToRuntimeGenerator = toRuntimeGenerator,
						Storage = storage,
						NativeStorage = new LocalStorage(gen, nativeLocal),
					};
				}).ToList();

			// marshal in parameters
			foreach (var paramInfo in paramInfos)
			{
				if (paramInfo.ToNativeGenerator != null)
				{
					paramInfo.ToNativeGenerator.EmitToNative(paramInfo.Storage, paramInfo.NativeStorage);
				}
			}

			// push the interface pointer
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ldfld, ptrFieldInfo);

			// push each parameters
			foreach (var paramInfo in paramInfos)
			{
				if (paramInfo.ToRuntimeGenerator != null)
				{
					// out/inout parameter
					paramInfo.NativeStorage.EmitLoadAddress();
				}
				else
				{
					paramInfo.NativeStorage.EmitLoad();
				}
			}			

			// get the interface pointer
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ldfld, ptrFieldInfo);

			// get vtable
			gen.Emit(OpCodes.Ldind_I);

			// load the vtable element
			int vtableIndex = comMethodInfo.VTableOffset;
			if (vtableIndex > 0)
			{
				gen.Emit(OpCodes.Sizeof, typeof(IntPtr));
				if (vtableIndex > 1)
				{
					gen.Emit(OpCodes.Ldc_I4, vtableIndex);
					gen.Emit(OpCodes.Mul);
				}
				gen.Emit(OpCodes.Conv_I);
				gen.Emit(OpCodes.Add);
			}
			gen.Emit(OpCodes.Ldind_I);

			// call!
			gen.EmitCalli(OpCodes.Calli, CallingConventions.Standard,
			              comMethodInfo.NativeReturnType, paramInfos.Select((p) => p.Marshaller.NativeParameterType).ToArray(), null);

			if (comMethodInfo.ReturnsHresult)
			{
				// handle COM error
				gen.EmitCall(OpCodes.Call, throwExceptionForHRMethod, null);
			}

			if (comMethodInfo.ReturnValueMarshaller != null)
			{
				if (comMethodInfo.ReturnsHresult)
				{
					throw new InvalidOperationException();
				}

				// marshal return value
				var nativeLocal = gen.DeclareLocal(comMethodInfo.NativeReturnType);
				gen.Emit(OpCodes.Stloc, nativeLocal);

				comMethodInfo.ReturnValueMarshaller.CreateToRuntimeGenerator(gen)
							 .EmitToRuntime(new LocalStorage(gen, nativeLocal),
											new LocalStorage(gen, returnValueLocal));
			}

			// marshal "out" parameters and optionally return value parameter
			foreach (var paramInfo in paramInfos)
			{
				if (paramInfo.ToRuntimeGenerator != null)
				{
					paramInfo.ToRuntimeGenerator.EmitToRuntime(paramInfo.NativeStorage, paramInfo.Storage);
				}
			}

			// return
			if (methodInfo.ReturnType != typeof(void))
			{
				gen.Emit(OpCodes.Ldloc, returnValueLocal);
			}
			gen.Emit(OpCodes.Ret);
		}

	}
}
