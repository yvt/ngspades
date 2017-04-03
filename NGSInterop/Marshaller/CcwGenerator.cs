using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
	// TODO: the signature of CCWFactory needs to be changed
	public delegate IntPtr CcwFactory<in T>(T interfaceObj) where T : class, IUnknown;

	struct CcwFactoryInfo<T> where T : class, IUnknown
	{
		public TypeInfo ClassTypeInfo { get; set; }
		public MethodInfo FactoryMethodInfo { get; set; }
		public CcwFactory<T> FactoryDelegate { get; set; }
	}

	public class CcwGenerator
	{
		ModuleBuilder moduleBuilder;

		public CcwGenerator(ModuleBuilder moduleBuilder)
		{
			this.moduleBuilder = moduleBuilder;
		}

		// TODO: CCWGenerator
	}
}
