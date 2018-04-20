//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Reflection;
namespace Ngs.Interop {
    public static class DelegateRuntimeInfo<T> where T : class {
        static T rcfpw;

        static DelegateRuntimeInfo() {
            if (typeof(T).GetTypeInfo().BaseType != typeof(MulticastDelegate)) {
                throw new InvalidOperationException($"Type {typeof(T).FullName} is not a delegate.");
            }
        }

        public static T RcfpwDelegate {
            get {
                // it's very okay to make a duplicate instance
                T ov = rcfpw;
                T ret = rcfpw ?? (rcfpw = DynamicModuleInfo.Instance.RcwGenerator.CreateRcfpw<T>().Delegate);

                // debug
                if (false && ov != rcfpw) {
                    var asm = DynamicModuleInfo.Instance.AssemblyBuilder;
                    var saveMethod = asm.GetType().GetRuntimeMethod("Save", new Type[] { typeof(string) });
                    if (saveMethod != null) {
                        saveMethod.Invoke(asm, new object[] { "DebugOutput.dll" });
                    }
                }

                return ret;
            }
        }
    }
}