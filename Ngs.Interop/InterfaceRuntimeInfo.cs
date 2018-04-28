//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
#if false
using System.Reflection;
#endif
using System.Collections.Generic;
namespace Ngs.Interop {
    /// <summary>
    /// NgsCOM infrastructure. Not intended to be used by an application.
    /// </summary>
    /// <typeparam name="T">An NgsCOM interface.</typeparam>
    public static class InterfaceRuntimeInfo<T> where T : class, IUnknown {
        private static readonly Marshaller.InterfaceInfo info = new Marshaller.InterfaceInfo(typeof(T));
        // private static readonly Dictionary<IntPtr, WeakReference<INativeObject<T>>> rcwInstances = new Dictionary<IntPtr, WeakReference<INativeObject<T>>>();
        private static Marshaller.RcwFactory<T> rcwFactory;

        /// <summary>
        /// The GUID of the interface.
        /// </summary>
        public static Guid ComGuid => info.ComGuid;

        /*
        // FIXME: Re-enable RCW caching after leak is solved.
                  Currently `ForgetRcw` has to be called manually!
                  (Also, RCW caching involves a global lock which affects the
                  performance negatively)
        public static void ForgetRcw(INativeObject<T> obj) {
            lock (rcwInstances) {
                WeakReference<INativeObject<T>> rcwref;
                INativeObject<T> rcw;
                if (rcwInstances.TryGetValue(obj.NativeInterfacePtr, out rcwref) &&
                    rcwref.TryGetTarget(out rcw) &&
                    rcw == obj) {
                    rcwInstances.Remove(obj.NativeInterfacePtr);
                }
            }
        } */

        internal static T CreateRcw(IntPtr interfacePtr, bool addRef) {
            /* lock (rcwInstances) */
            {
                if (rcwFactory == null) {
                    rcwFactory = DynamicModuleInfo.Instance.RcwGenerator.CreateRcwFactory<T>().FactoryDelegate;

                    // debug
#if false
                    var asm = DynamicModuleInfo.Instance.AssemblyBuilder;
                    var saveMethod = asm.GetType ().GetRuntimeMethod ("Save", new Type[] { typeof (string) });
                    if (saveMethod != null) {
                        saveMethod.Invoke (asm, new object[] { "DebugOutput.dll" });
                    }
#endif
                }

                // WeakReference<INativeObject<T>> rcwref;
                INativeObject<T> rcw;

                /*
                if (rcwInstances.TryGetValue(interfacePtr, out rcwref) &&
                    rcwref.TryGetTarget(out rcw)) {
                    if (!addRef) {
                        rcw.Release();
                    }
                    return rcw.Interface;
                }
                */

                rcw = rcwFactory(interfacePtr);
                if (!addRef) {
                    rcw.Release();
                }
                // rcwInstances[interfacePtr] = new WeakReference<INativeObject<T>>(rcw);

                return rcw.Interface;
            }
        }
    }
}