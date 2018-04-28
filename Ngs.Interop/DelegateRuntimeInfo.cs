//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Reflection;
namespace Ngs.Interop {
    /// <summary>
    /// NgsCOM infrastructure. Not intended to be used by an application.
    /// </summary>
    /// <typeparam name="T">A delegate type.</typeparam>
    public static class DelegateRuntimeInfo<T> where T : class {
        static T rcfpw;

        static DelegateRuntimeInfo() {
            if (typeof(T).GetTypeInfo().BaseType != typeof(MulticastDelegate)) {
                throw new InvalidOperationException($"Type {typeof(T).FullName} is not a delegate.");
            }
        }

        /// <summary>
        /// NgsCOM infrastructure. Not intended to be used by an application.
        /// </summary>
        public static T RcfpwDelegate {
            get {
                // it's very okay to make a duplicate instance
                T ov = rcfpw;
                T ret = rcfpw ?? (rcfpw = DynamicModuleInfo.Instance.RcwGenerator.CreateRcfpw<T>().Delegate);

#if false
                // debug
                if (ov != rcfpw) {
                    var asm = DynamicModuleInfo.Instance.AssemblyBuilder;
                    var saveMethod = asm.GetType().GetRuntimeMethod("Save", new Type[] { typeof(string) });
                    if (saveMethod != null) {
                        saveMethod.Invoke(asm, new object[] { "DebugOutput.dll" });
                    }
                }
#endif

                return ret;
            }
        }
    }
}