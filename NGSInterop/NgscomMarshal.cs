//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
namespace Ngs.Interop
{
    /// <summary>
    /// Provides a collection of method for interacting unmanaged code based on
    /// the NgsCOM ABI.
    /// </summary>
    public static partial class NgscomMarshal
    {
        [System.Security.SecurityCritical]
        public static T GetRcwForInterfacePtr<T>(IntPtr ptr, bool addRef) where T : class, IUnknown
        {
            if (ptr == IntPtr.Zero)
            {
                return null;
            }
            return InterfaceRuntimeInfo<T>.CreateRcw(ptr, addRef);
        }

        [System.Security.SecurityCritical]
        public static IntPtr GetCcwForInterface<T>(T obj) where T : class, IUnknown
        {
            if (obj == null)
            {
                return IntPtr.Zero;
            }

            var nativeObj = obj as INativeObject<T>;
            if (nativeObj != null)
            {
                nativeObj.AddRef();
                return nativeObj.NativeInterfacePtr;
            }

            var guid = InterfaceRuntimeInfo<T>.ComGuid;
            return obj.QueryNativeInterface(ref guid);
        }

        public static T QueryInterfaceOrNull<T>(IUnknown obj) where T : class, IUnknown
        {
            // FIXME: rewrite this
            var nativeObj = obj as INativeObject<T>;
            if (nativeObj != null)
            {
                try
                {
                    var guid = InterfaceRuntimeInfo<T>.ComGuid;
                    var ptr = nativeObj.QueryNativeInterface(ref guid);
                    return InterfaceRuntimeInfo<T>.CreateRcw(ptr, true);
                }
                catch (System.Runtime.InteropServices.COMException ex) // TODO: use C# 7.0 filter clause
                {
                    const int E_NOINTERFACE = unchecked((int)0x80004002);
                    if (ex.HResult == E_NOINTERFACE)
                    {
                        return null;
                    }
                    else
                    {
                        throw;
                    }
                }
            }
            return obj as T;
        }

        public static T QueryInterface<T>(IUnknown obj) where T : class, IUnknown
        {
            // FIXME: rewrite this
            var nativeObj = obj as INativeObject<T>;
            if (nativeObj != null)
            {
                var guid = InterfaceRuntimeInfo<T>.ComGuid;
                var ptr = nativeObj.QueryNativeInterface(ref guid);
                return InterfaceRuntimeInfo<T>.CreateRcw(ptr, true);
            }
            return (T)obj;
        }
    }
}
