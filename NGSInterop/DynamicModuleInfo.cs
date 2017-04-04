using System;
using System.Reflection;
using System.Reflection.Emit;
namespace Ngs.Interop
{
	class DynamicModuleInfo
	{
		static DynamicModuleInfo instance;

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
			AssemblyBuilder = AssemblyBuilder.DefineDynamicAssembly(asmName, AssemblyBuilderAccess.RunAndCollect);

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
	}
}
