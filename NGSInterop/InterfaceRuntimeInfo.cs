using System;
using System.Linq;
using System.Reflection;
using System.Collections.Generic;
namespace Ngs.Interop
{
	public static class InterfaceRuntimeInfo<T> where T : class, IUnknown
	{
		private static readonly Marshaller.InterfaceInfo info = new Marshaller.InterfaceInfo(typeof(T));
		private static readonly Dictionary<IntPtr, WeakReference<INativeObject<T>>> rcwInstances = new Dictionary<IntPtr, WeakReference<INativeObject<T>>>();
		private static Marshaller.RcwFactory<T> rcwFactory;

		public static Guid ComGuid => info.ComGuid;

		public static void ForgetRcw(INativeObject<T> obj)
		{
			lock (rcwInstances)
			{
				WeakReference<INativeObject<T>> rcwref;
				INativeObject<T> rcw;
				if (rcwInstances.TryGetValue(obj.NativeInterfacePtr, out rcwref) &&
				    rcwref.TryGetTarget(out rcw) &&
				    rcw == obj)
				{
					rcwInstances.Remove(obj.NativeInterfacePtr);
				}
			}
		}

		internal static T CreateRcw(IntPtr interfacePtr, bool addRef)
		{
			lock (rcwInstances)
			{
				if (rcwFactory == null)
				{
					rcwFactory = DynamicModuleInfo.Instance.RcwGenerator.CreateRcwFactory<T>().FactoryDelegate;

					// debug
					if (false) {
						var asm = DynamicModuleInfo.Instance.AssemblyBuilder;
						var saveMethod = asm.GetType().GetRuntimeMethod("Save", new Type[] {typeof(string)});
						if (saveMethod != null) {
							saveMethod.Invoke(asm, new object[] {"DebugOutput.dll"});
						}
					}
				}

				WeakReference<INativeObject<T>> rcwref;
				INativeObject<T> rcw;

				if (rcwInstances.TryGetValue(interfacePtr, out rcwref) &&
					rcwref.TryGetTarget(out rcw))
				{
					if (!addRef)
					{
						rcw.Release();
					}
					return rcw.Interface;
				}

				rcw = rcwFactory(interfacePtr);
				if (!addRef)
				{
					rcw.Release();
				}
				rcwInstances[interfacePtr] = new WeakReference<INativeObject<T>>(rcw);

				return rcw.Interface;
			}
		}
	}
}
