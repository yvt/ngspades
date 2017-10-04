using System;
using System.Reflection;
using System.Reflection.Emit;
namespace Ngs.Interop
{
	class DynamicModuleInfo
	{
		internal static DynamicModuleInfo instance;

		public static DynamicModuleInfo Instance
		{
			get
			{
				// it won't do harm to make a duplicate
				if (instance == null)
				{
					return instance = new DynamicModuleInfo();
				}
				return instance;
			}
		}

		public AssemblyBuilder AssemblyBuilder { get; }
		public ModuleBuilder ModuleBuilder { get; }
		public Marshaller.RcwGenerator RcwGenerator { get; }
		public Marshaller.CcwGenerator CcwGenerator { get; }

		private DynamicModuleInfo()
		{
			var asmName = new AssemblyName("NGSInteropDynamicAssembly");
			var asmBuilderAccess = IsGetFunctionPtrForDelegateSupportedOnRunAndCollectAssembly() ?
				AssemblyBuilderAccess.RunAndCollect : AssemblyBuilderAccess.Run;
			AssemblyBuilder = AssemblyBuilder.DefineDynamicAssembly(asmName, asmBuilderAccess);

			// Use one of the overloads of GetRuntimeMethod with a dll name argument to ease
			// the debug if possible
			var method = AssemblyBuilder.GetType().GetRuntimeMethod("DefineDynamicModule",
				new [] { typeof(string), typeof(string) });
			if (method != null) {
				ModuleBuilder = (ModuleBuilder) method.Invoke(AssemblyBuilder, new object[] {"PinkiePie", "DebugOutput.dll"});
			} else {
				ModuleBuilder = AssemblyBuilder.DefineDynamicModule("PinkiePie");
			}

			RcwGenerator = new Marshaller.RcwGenerator(ModuleBuilder);
			CcwGenerator = new Marshaller.CcwGenerator(ModuleBuilder);
		}

		// .NET core is strict about this
		static bool IsGetFunctionPtrForDelegateSupportedOnRunAndCollectAssembly()
		{
			try {
				var asmName = new AssemblyName("CheckIsGetFunctionPtrFromDelegateSupportedOnRunAndCollectAssemblyAssembly");
				var asmBuilder = AssemblyBuilder.DefineDynamicAssembly(asmName, AssemblyBuilderAccess.RunAndCollect);
				var modBuilder = asmBuilder.DefineDynamicModule("SomeModule");

				var delegateType = modBuilder.DefineType(
					"Lorenzs",
					TypeAttributes.Sealed | TypeAttributes.Public,
					typeof(MulticastDelegate));
						
				var ctor = delegateType.DefineConstructor(
					MethodAttributes.RTSpecialName | MethodAttributes.HideBySig | MethodAttributes.Public,
					CallingConventions.Standard, new[] { typeof(object), typeof(IntPtr) });
				ctor.SetImplementationFlags(MethodImplAttributes.Runtime | MethodImplAttributes.Managed);

				var invokeMethod = delegateType.DefineMethod(
					"Invoke", MethodAttributes.HideBySig | MethodAttributes.Virtual | MethodAttributes.Public,
					delegateType, new Type[] {});
				invokeMethod.SetImplementationFlags(MethodImplAttributes.Runtime | MethodImplAttributes.Managed);

				var testMethod = delegateType.DefineMethod(
					"Test", MethodAttributes.Public | MethodAttributes.Static,
					delegateType, new Type[] {});
				
				var gen = testMethod.GetILGenerator();
				gen.Emit(OpCodes.Ldnull);
				gen.Emit(OpCodes.Ldftn, testMethod);
				gen.Emit(OpCodes.Newobj, ctor);
				gen.Emit(OpCodes.Ret);

				var typeInfo = delegateType.CreateTypeInfo();

				var factory = typeInfo.GetDeclaredMethod("Test");

				System.Runtime.InteropServices.Marshal.GetFunctionPointerForDelegate<Delegate>
				((Delegate)factory.Invoke(null, new object[] {}));
		
				return true;
			} catch (NotSupportedException) {
				return false;
			}
		}
	}
}
