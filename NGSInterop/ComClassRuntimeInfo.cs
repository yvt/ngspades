using System;
using System.Runtime.InteropServices;
using System.Linq;
using System.Reflection;
using System.Collections.Generic;

namespace Ngs.Interop
{
    class CcwVtableCache
    {
		public Tuple<Delegate[], IntPtr[]> VtableWithDelegates { get; }

        public CcwVtableCache(Type baseInterface)
        {
            var factory = DynamicModuleInfo.Instance.CcwGenerator.CreateCcwFactory(baseInterface).FactoryDelegate;

            // debug
            if (false) {
                var asm = DynamicModuleInfo.Instance.AssemblyBuilder;
                var saveMethod = asm.GetType().GetRuntimeMethod("Save", new Type[] {typeof(string)});
                if (saveMethod != null) {
                    saveMethod.Invoke(asm, new object[] {"DebugOutput.dll"});
                    throw new Exception("abort!!!");
                }
            }

            var delegates = factory();

            var fptrs = delegates.Select((fn) => Marshal.GetFunctionPointerForDelegate<Delegate>(fn)).ToArray();

            VtableWithDelegates = Tuple.Create(delegates, fptrs);
        }
    }

    // generic class + static field = WeakMap<Type, T>
    static class CcwVtableCache<T> where T : class, IUnknown
    {
        public static readonly CcwVtableCache Instance = new CcwVtableCache(typeof(T));
    }

    class ComClassHeaderInfo
    {
        public IntPtr VTablePtr => vtableHandle.AddrOfPinnedObject();
        public Guid[] Guids { get; }

        GCHandle vtableHandle;
        Delegate[] vtableDelegates;

        public ComClassHeaderInfo(Type baseInterface)
        {
            var cacheType = typeof(CcwVtableCache<>).MakeGenericType(new [] {baseInterface});
            var instanceField = cacheType.GetRuntimeField("Instance");
            var cache = (CcwVtableCache) instanceField.GetValue(null);
            var vtableWithDelegates = cache.VtableWithDelegates;
            
            vtableDelegates = vtableWithDelegates.Item1;
            IntPtr[] vtable = vtableWithDelegates.Item2;
            vtableHandle = GCHandle.Alloc(vtable, GCHandleType.Pinned);

            Guids = new Marshaller.InterfaceInfo(baseInterface).AllImplementedInterfaces
                .Select((iface) => new Marshaller.InterfaceInfo(iface).ComGuid)
                .ToArray();
        }

        ~ComClassHeaderInfo()
        {
            vtableHandle.Free();
        }
    }

    sealed class ComClassRuntimeInfo
    {
        public ComClassHeaderInfo[] Headers { get; }

        public ComClassRuntimeInfo(Type t)
        {
            var headers = new List<ComClassHeaderInfo>();
            foreach (var iface in t.GetTypeInfo().ImplementedInterfaces)
            {
                if (!typeof(IUnknown).GetTypeInfo().IsAssignableFrom(iface.GetTypeInfo()))
                {
                    continue;
                }
                headers.Add(new ComClassHeaderInfo(iface));
            }
            Headers = headers.ToArray();
        }
    }

    sealed class ComClassRuntimeInfo<T> where T : ComClass
    {
        public static readonly ComClassRuntimeInfo Instance = new ComClassRuntimeInfo(typeof(T));
    }
}