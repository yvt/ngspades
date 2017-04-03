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

		public ModuleBuilder ModuleBuilder { get; }
		public Marshaller.RcwGenerator RcwGenerator { get; }
		public Marshaller.CcwGenerator CcwGenerator { get; }

		private DynamicModuleInfo()
		{
			var asmName = new AssemblyName("");
			var asmBuilder = AssemblyBuilder.DefineDynamicAssembly(asmName, AssemblyBuilderAccess.RunAndCollect);
			ModuleBuilder = asmBuilder.DefineDynamicModule("pinkiepie.dll");
			RcwGenerator = new Marshaller.RcwGenerator(ModuleBuilder);
			CcwGenerator = new Marshaller.CcwGenerator(ModuleBuilder);
		}
	}
}
