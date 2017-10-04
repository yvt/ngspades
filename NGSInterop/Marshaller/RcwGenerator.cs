using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;
using System.Security;

namespace Ngs.Interop.Marshaller
{
	[SecurityCriticalAttribute]
	public delegate INativeObject<T> RcwFactory<out T>(IntPtr interfacePtr) where T : class, IUnknown;

	struct RcwFactoryInfo<T> where T : class, IUnknown
	{
		public TypeInfo ClassTypeInfo { get; set; }
		public MethodInfo FactoryMethodInfo { get; set; }
		public RcwFactory<T> FactoryDelegate { get; set; }
	}

	/**
	 * Contains information about a RCFPW (runtime callable function pointer wrapper).
	 */
	struct RcfpwInfo<T> where T : class
	{
		public TypeInfo ClassTypeInfo { get; set; }
		public MethodInfo MethodInfo { get; set; }
		public T Delegate { get; set; }
	}

	sealed class RcwGenerator
	{
		static readonly MethodInfo throwExceptionForHRMethod =
			typeof(Marshal).GetRuntimeMethod(nameof(Marshal.ThrowExceptionForHR), new Type[] { typeof(int) });

		ModuleBuilder moduleBuilder;
		Dictionary<Type, RcwFactoryInfo<IUnknown>> rcws = new Dictionary<Type, RcwFactoryInfo<IUnknown>>();
		Dictionary<Type, RcfpwInfo<object>> rcfpws = new Dictionary<Type, RcfpwInfo<object>>();

		public RcwGenerator(ModuleBuilder moduleBuilder)
		{
			this.moduleBuilder = moduleBuilder;
		}

		public RcfpwInfo<object> CreateRcfpw(Type delegateType)
		{
			RcfpwInfo<object> rcfpw;
			if (!rcfpws.TryGetValue(delegateType, out rcfpw))
			{
				rcfpw = new RcfpwInfo<object>();
				rcfpw.ClassTypeInfo = CreateRcfpwClass(delegateType);
				rcfpw.MethodInfo = rcfpw.ClassTypeInfo.GetDeclaredMethod("Invoke");
				rcfpw.Delegate = rcfpw.MethodInfo.CreateDelegate(delegateType);
				rcfpws[delegateType] = rcfpw;
			}
			return rcfpw;
		}

		public RcfpwInfo<T> CreateRcfpw<T>() where T : class
		{
			var info = CreateRcfpw(typeof(T));
			return new RcfpwInfo<T>() {
				ClassTypeInfo = info.ClassTypeInfo,
				MethodInfo = info.MethodInfo,
				Delegate = (T)info.Delegate
			};
		}

		public RcwFactoryInfo<IUnknown> CreateRcwFactory(Type interfaceType)
		{
			RcwFactoryInfo<IUnknown> rcw;
			if (!rcws.TryGetValue(interfaceType, out rcw))
			{
				rcw = new RcwFactoryInfo<IUnknown>();
				rcw.ClassTypeInfo = CreateRCWClass(new InterfaceInfo(interfaceType));
				rcw.FactoryMethodInfo = rcw.ClassTypeInfo.GetDeclaredMethod("Create");
				rcw.FactoryDelegate = (RcwFactory<IUnknown>)
					rcw.FactoryMethodInfo.CreateDelegate(typeof(RcwFactory<>).MakeGenericType(interfaceType));
				rcws[interfaceType] = rcw;
			}
			return rcw;
		}

		public RcwFactoryInfo<T> CreateRcwFactory<T>() where T : class, IUnknown
		{
			var info = CreateRcwFactory(typeof(T));
			return new RcwFactoryInfo<T>()
			{
				ClassTypeInfo = info.ClassTypeInfo,
				FactoryMethodInfo = info.FactoryMethodInfo,
				FactoryDelegate = (RcwFactory<T>) info.FactoryDelegate
			};
		}

		TypeInfo CreateRCWClass(InterfaceInfo interfaceInfo)
		{
			var typeBuilder = moduleBuilder.DefineType(interfaceInfo.Type.FullName + "<RCW>",
			                                           TypeAttributes.Class | TypeAttributes.Sealed |
			                                           TypeAttributes.NotPublic);
			
			DefineRcwClass(interfaceInfo, typeBuilder);
			return typeBuilder.CreateTypeInfo();
		}

		TypeInfo CreateRcfpwClass(Type delegateType)
		{
			if (delegateType.GetTypeInfo().BaseType != typeof(MulticastDelegate))
			{
				throw new ArgumentException("Not a delegate.", nameof(delegateType));
			}
			if (delegateType.IsConstructedGenericType)
			{
				throw new ArgumentException("Generic delegate type isn't supported yet.", nameof(delegateType));
			}

			var typeBuilder = moduleBuilder.DefineType(delegateType.FullName + "<RCFPW>",
			                                           TypeAttributes.Class | TypeAttributes.Sealed |
			                                           TypeAttributes.NotPublic);
			
			var invokeMethod = delegateType.GetRuntimeMethods().First((m) => m.Name == "Invoke");
			var mi = new ComMethodInfo(invokeMethod, 0, ComMethodType.FunctionPtrThunk);
			CreateFunctionPtrWrapperMethod(mi, typeBuilder);

			return typeBuilder.CreateTypeInfo();
		}

		static readonly PropertyInfo nativeInterfacePtrProp = typeof(INativeObject<>)
			.GetRuntimeProperty(nameof(INativeObject<IUnknown>.NativeInterfacePtr));
		static readonly PropertyInfo nativeIUnknownPtrProp = typeof(INativeObject<>)
			.GetRuntimeProperty(nameof(INativeObject<IUnknown>.NativeIUnknownPtr));
		static readonly PropertyInfo interfaceProp = typeof(INativeObject<>)
			.GetRuntimeProperty(nameof(INativeObject<IUnknown>.Interface));

		static void DefineRcwClass(InterfaceInfo interfaceInfo, TypeBuilder typeBuilder)
		{
			var type = typeBuilder;

			var nativeObjectType = typeof(INativeObject<>).GetTypeInfo().MakeGenericType(type);
			var nativeObjectTypeInfo = nativeObjectType.GetTypeInfo();

			typeBuilder.AddInterfaceImplementation(nativeObjectType);

			foreach (var theInterface in interfaceInfo.AllImplementedInterfaces)
			{
				typeBuilder.AddInterfaceImplementation(theInterface);
			}

			FieldInfo interfacePtrField = typeBuilder.DefineField("interfacePtr", typeof(IntPtr), 0);
			FieldInfo unknownPtrField = typeBuilder.DefineField("iUnknownPtr", typeof(IntPtr), 0);

			// generate constructor
			var ctor = CreateRcwConstructor(typeBuilder, interfacePtrField, unknownPtrField);

			// generate finalizer
			CreateRcwFinalizer(typeBuilder);

			// generate factory
			CreateRcwFactoryMethod(typeBuilder, ctor);

			var methodNameUniqifier = new Utils.UniqueNameGenerator();
			methodNameUniqifier.Uniquify("interfacePtr");
			methodNameUniqifier.Uniquify("iUnknownPtr");
			methodNameUniqifier.Uniquify("Finalize");
			methodNameUniqifier.Uniquify("Create");
			foreach (var method in interfaceInfo.ComMethodInfos)
			{
				CreateRcwMethod(method, typeBuilder, interfacePtrField, methodNameUniqifier);
			}

			// implement INativeObject
			ImplementFieldGetter(typeBuilder, interfacePtrField,
								 nativeInterfacePtrProp, TypeBuilder.GetMethod(nativeObjectType, nativeInterfacePtrProp.GetMethod),
								 methodNameUniqifier);
			ImplementFieldGetter(typeBuilder, unknownPtrField,
								 nativeIUnknownPtrProp, TypeBuilder.GetMethod(nativeObjectType, nativeIUnknownPtrProp.GetMethod),
								 methodNameUniqifier);
			ImplementInterfaceProperty(typeBuilder, 
								 interfaceProp, TypeBuilder.GetMethod(nativeObjectType, interfaceProp.GetMethod),
								 methodNameUniqifier);
		}

		static readonly MethodInfo comGuidGetter = typeof(InterfaceRuntimeInfo<IUnknown>).GetTypeInfo()
			.GetDeclaredProperty(nameof(InterfaceRuntimeInfo<IUnknown>.ComGuid)).GetMethod;
		static readonly MethodInfo addRefMethod = typeof(IUnknown).GetTypeInfo()
			.GetDeclaredMethod(nameof(IUnknown.AddRef));
		static readonly MethodInfo releaseMethod = typeof(IUnknown).GetTypeInfo()
			.GetDeclaredMethod(nameof(IUnknown.Release));
		static readonly MethodInfo queryNativeInterfaceMethod = typeof(IUnknown).GetTypeInfo()
			.GetDeclaredMethod(nameof(IUnknown.QueryNativeInterface));
		
		static ConstructorBuilder CreateRcwConstructor(TypeBuilder typeBuilder, FieldInfo interfacePtrField, FieldInfo unknownPtrField)
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

			// get IUnknown's guid
			var iunkLocal = gen.DeclareLocal(typeof(Guid));
			gen.Emit(OpCodes.Call, comGuidGetter); // get IUnknown guid
			gen.Emit(OpCodes.Stloc, iunkLocal);

			// initialize this.iUnknownPtr
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ldloca, iunkLocal);
			gen.Emit(OpCodes.Callvirt, queryNativeInterfaceMethod);
			gen.Emit(OpCodes.Stfld, unknownPtrField);

			// Reset the ref count which was incremented by QueryNativeInterface
			gen.Emit(OpCodes.Ldarg_0);
			gen.EmitCall(OpCodes.Callvirt, releaseMethod, null);
			gen.Emit(OpCodes.Pop);

			gen.Emit(OpCodes.Ret);

			return ctorBuilder;
		}

		static readonly CustomAttributeBuilder securityCriticalAttributeCab = 
			new CustomAttributeBuilder(typeof(SecurityCriticalAttribute).GetTypeInfo().DeclaredConstructors.First((ctor) => ctor.GetParameters().Length == 0),
				new object[] {});

		static MethodBuilder CreateRcwFactoryMethod(TypeBuilder typeBuilder, ConstructorBuilder ctorInfo)
		{
			var nativeObjectType = typeof(INativeObject<>).GetTypeInfo().MakeGenericType(typeBuilder);
			var methodBuilder = typeBuilder.DefineMethod("Create",
			                                             MethodAttributes.Static | MethodAttributes.Public | MethodAttributes.HideBySig,
														 CallingConventions.Standard,
														 nativeObjectType, new Type[] { typeof(IntPtr) });

			methodBuilder.SetCustomAttribute(securityCriticalAttributeCab);

			var gen = methodBuilder.GetILGenerator();
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Newobj, ctorInfo);
			gen.Emit(OpCodes.Castclass, nativeObjectType);
			gen.Emit(OpCodes.Ret);

			return methodBuilder;
		}

		static void CreateRcwFinalizer(TypeBuilder typeBuilder)
		{
			var methodBuilder = typeBuilder.DefineMethod("Finalize",
			                                             MethodAttributes.Virtual | MethodAttributes.Family | MethodAttributes.HideBySig,
														 CallingConventions.Standard,
														 typeof(void), new Type[] { });

			var gen = methodBuilder.GetILGenerator();

			gen.Emit(OpCodes.Ldarg_0);
			gen.EmitCall(OpCodes.Callvirt, releaseMethod, null);
			gen.Emit(OpCodes.Pop);
			gen.Emit(OpCodes.Ret);
		}

		static void ImplementInterfaceProperty(TypeBuilder typeBuilder, PropertyInfo genericPropertyInfo, MethodInfo getterMethodInfo,
										 Utils.UniqueNameGenerator methodNameUniqifier)
		{
			var propNameTemplate = getterMethodInfo.DeclaringType.Name + "." + genericPropertyInfo.Name;
			var propName = methodNameUniqifier.Uniquify(propNameTemplate);
			var prop = typeBuilder.DefineProperty(propName, PropertyAttributes.None,
				getterMethodInfo.ReturnType, new Type[] {});

			var methodNameTemplate = getterMethodInfo.DeclaringType.Name + "." + getterMethodInfo.Name;
			var methodName = methodNameUniqifier.Uniquify(methodNameTemplate);
			var methodBuilder = typeBuilder.DefineMethod(methodName,
														 MethodAttributes.Virtual | MethodAttributes.SpecialName | MethodAttributes.HideBySig,
														 CallingConventions.Standard,
														 typeBuilder,
														 new Type[] { });
			typeBuilder.DefineMethodOverride(methodBuilder, getterMethodInfo);

			var gen = methodBuilder.GetILGenerator();
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ret);

			prop.SetGetMethod(methodBuilder);
		}

		static void ImplementFieldGetter(TypeBuilder typeBuilder, FieldInfo fieldInfo, PropertyInfo genericPropertyInfo, MethodInfo getterMethodInfo,
		                                 Utils.UniqueNameGenerator methodNameUniqifier)
		{
			var propNameTemplate = getterMethodInfo.DeclaringType.Name + "." + genericPropertyInfo.Name;
			var propName = methodNameUniqifier.Uniquify(propNameTemplate);
			var prop = typeBuilder.DefineProperty(propName, PropertyAttributes.None,
				getterMethodInfo.ReturnType, new Type[] {});

			var methodNameTemplate = getterMethodInfo.DeclaringType.Name + "." + getterMethodInfo.Name;
			var methodName = methodNameUniqifier.Uniquify(methodNameTemplate);
			var methodBuilder = typeBuilder.DefineMethod(methodName,
														 MethodAttributes.Virtual | MethodAttributes.SpecialName | MethodAttributes.HideBySig,
														 CallingConventions.Standard,
														 getterMethodInfo.ReturnType,
														 new Type[] { });
			typeBuilder.DefineMethodOverride(methodBuilder, getterMethodInfo);

			var gen = methodBuilder.GetILGenerator();
			gen.Emit(OpCodes.Ldarg_0);
			gen.Emit(OpCodes.Ldfld, fieldInfo);
			gen.Emit(OpCodes.Ret);

			prop.SetGetMethod(methodBuilder);
		}

		static void CreateRcwMethod(ComMethodInfo comMethodInfo, TypeBuilder typeBuilder,
									FieldInfo ptrFieldInfo, Utils.UniqueNameGenerator methodNameUniqifier)
		{
			var methodInfo = comMethodInfo.MethodInfo;
			var methodNameTemplate = comMethodInfo.MethodInfo.DeclaringType.Name + "." + methodInfo.Name;
			var methodName = methodNameUniqifier.Uniquify(methodNameTemplate);
			var methodBuilder = typeBuilder.DefineMethod(methodName,
														 MethodAttributes.Virtual | MethodAttributes.HideBySig, CallingConventions.Standard,
														 methodInfo.ReturnType,
														 methodInfo.GetParameters().Select((p) => p.ParameterType).ToArray());
			typeBuilder.DefineMethodOverride(methodBuilder, methodInfo);

			// define a method
			foreach (var parameter in methodInfo.GetParameters())
			{
				methodBuilder.DefineParameter(parameter.Position + 1,
					parameter.Attributes, parameter.Name);
			}

			CreateRCWMethodBody(comMethodInfo, ptrFieldInfo, methodBuilder);
		}

		static void CreateFunctionPtrWrapperMethod(ComMethodInfo comMethodInfo, TypeBuilder typeBuilder)
		{
			var methodInfo = comMethodInfo.MethodInfo;
			var methodBuilder = typeBuilder.DefineMethod("Invoke",
														 MethodAttributes.Static | MethodAttributes.HideBySig, CallingConventions.Standard,
														 methodInfo.ReturnType,
														 methodInfo.GetParameters().Select((p) => p.ParameterType).ToArray());
				
			// define a method
			foreach (var parameter in methodInfo.GetParameters())
			{
				methodBuilder.DefineParameter(parameter.Position + 1,
					parameter.Attributes, parameter.Name);
			}

			CreateRCWMethodBody(comMethodInfo, null, methodBuilder);
		}

		static void CreateRCWMethodBody(ComMethodInfo comMethodInfo, FieldInfo ptrFieldInfo, MethodBuilder methodBuilder)
		{
			var methodInfo = comMethodInfo.MethodInfo;

			if ((ptrFieldInfo == null) != comMethodInfo.IsFirstParameterFunctionPtr) {
				throw new InvalidOperationException();
			}

			// generate a method body
			var gen = methodBuilder.GetILGenerator();

			LocalBuilder returnValueLocal = methodInfo.ReturnType != typeof(void) ?
															   gen.DeclareLocal(methodInfo.ReturnType) : null;

			bool isInstanceMethod = (methodBuilder.Attributes & MethodAttributes.Static) == 0;

			// create parameter marshallers
			var paramInfos = comMethodInfo.ParameterInfos
				.Select((p) =>
				{
					var marshaller = p.ValueMarshaller;
					var nativeLocal = gen.DeclareLocal(marshaller.NativeParameterType);
					var nativeLocalPin = p.IsOut ? gen.DeclareLocal(p.NativeType.MakePointerType(), true) : null;
					var toNativeGenerator = p.IsIn ? marshaller.CreateToNativeGenerator(gen) : null;
					var toRuntimeGenerator = p.IsOut ? marshaller.CreateToRuntimeGenerator(gen) : null;
					Storage storage;

					if (p.IsReturnValue)
					{
						storage = new LocalStorage(gen, returnValueLocal);
					}
					else if (p.IsOut)
					{
						storage = new ParameterStorage(gen, p.ParameterInfo.ParameterType, p.ParameterInfo.Position + (isInstanceMethod ? 1 : 0));
						storage = new IndirectStorage(storage);
					}
					else
					{
						storage = new ParameterStorage(gen, p.Type, p.ParameterInfo.Position + (isInstanceMethod ? 1 : 0));
					}

					return new
					{
						ParameterInfo = p,
						NativeLocal = nativeLocal,
						NativeLocalPin = nativeLocalPin,
						Marshaller = marshaller,
						ToNativeGenerator = toNativeGenerator,
						ToRuntimeGenerator = toRuntimeGenerator,
						Storage = storage,
						NativeStorage = new LocalStorage(gen, nativeLocal),
					};
				}).ToArray();

			// marshal in parameters
			foreach (var paramInfo in paramInfos)
			{
				if (paramInfo.ToNativeGenerator != null)
				{
					paramInfo.ToNativeGenerator.EmitToNative(paramInfo.Storage, paramInfo.NativeStorage);
				}
			}

			// pin by-ref parameters
			foreach (var paramInfo in paramInfos)
			{
				if (paramInfo.ParameterInfo.IsOut)
				{
					paramInfo.NativeStorage.EmitLoadAddress();
					gen.Emit(OpCodes.Stloc, paramInfo.NativeLocalPin);
				}
			}

			// push the interface pointer
			if (comMethodInfo.IsFirstNativeParameterInterfacePtr) {
				if (ptrFieldInfo == null) {
					throw new InvalidOperationException();
				}
				gen.Emit(OpCodes.Ldarg_0);
				gen.Emit(OpCodes.Ldfld, ptrFieldInfo);
			}

			// push each parameters
			foreach (var paramInfo in paramInfos)
			{
				if (paramInfo.ParameterInfo.IsOut)
				{
					// out/inout parameter
					gen.Emit(OpCodes.Ldloc, paramInfo.NativeLocalPin);
				}
				else
				{
					paramInfo.NativeStorage.EmitLoad();
				}
			}

			if (comMethodInfo.IsFirstParameterFunctionPtr) {
				// load the function ptr
				gen.Emit(OpCodes.Ldarg_0);
			} else {
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
			}

			var nativeParamTypes = new List<Type>();

			if (comMethodInfo.IsFirstNativeParameterInterfacePtr) {
				nativeParamTypes.Add(typeof(IntPtr));
			}

			nativeParamTypes.AddRange(paramInfos.Select((p) => p.ParameterInfo.IsOut ?
			 	p.Marshaller.NativeParameterType.MakePointerType() : p.Marshaller.NativeParameterType));

			// call!
			gen.EmitCalli(OpCodes.Calli, CallingConventions.Standard,
			              comMethodInfo.NativeReturnType, nativeParamTypes.ToArray(), null);
			if (comMethodInfo.ReturnsHresult)
			{
				// handle COM error
				// ThrowExceptionForHR is extremely slow (60~ cycles) on some environments (namely, Mono)
				// check the HRESULT before passing it to ThrowExceptionForHR
				var resultLocal = gen.DeclareLocal(typeof(int));
				gen.Emit(OpCodes.Dup);
				gen.Emit(OpCodes.Stloc, resultLocal);
				
				var noErrorLabel = gen.DefineLabel();
				gen.Emit(OpCodes.Ldc_I4_0);
				gen.Emit(OpCodes.Bge, noErrorLabel);

				// clean-up
				foreach (var paramInfo in paramInfos)
				{
					// destruct "out" parameters and optionally return value parameter
					if (paramInfo.ToRuntimeGenerator != null)
					{
						paramInfo.ToRuntimeGenerator.EmitDestructNativeValue(paramInfo.NativeStorage);
					}
					
					// destruct native values for "in" parameters
					if (!paramInfo.ParameterInfo.IsOut && paramInfo.ToNativeGenerator != null)
					{
						paramInfo.ToNativeGenerator.EmitDestructNativeValue(paramInfo.NativeStorage);	
					}
				}

				gen.Emit(OpCodes.Ldloc, resultLocal);
				gen.EmitCall(OpCodes.Call, throwExceptionForHRMethod, null);
				gen.MarkLabel(noErrorLabel);
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
											new LocalStorage(gen, returnValueLocal), move: true);
			}

			// unpin by-ref parameters
			foreach (var paramInfo in paramInfos)
			{
				if (paramInfo.ParameterInfo.IsOut)
				{
					gen.Emit(OpCodes.Ldc_I4_0);
					gen.Emit(OpCodes.Conv_I);
					gen.Emit(OpCodes.Stloc, paramInfo.NativeLocalPin);
				}
			}

			foreach (var paramInfo in paramInfos)
			{
				// marshal "out" parameters and optionally return value parameter
				if (paramInfo.ToRuntimeGenerator != null)
				{
					paramInfo.ToRuntimeGenerator.EmitToRuntime(paramInfo.NativeStorage, paramInfo.Storage, move: true);
				}

				// destruct native values for "in" parameters
				if (!paramInfo.ParameterInfo.IsOut && paramInfo.ToNativeGenerator != null)
				{
					paramInfo.ToNativeGenerator.EmitDestructNativeValue(paramInfo.NativeStorage);	
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
