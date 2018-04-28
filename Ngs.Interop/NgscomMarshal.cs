//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Runtime.InteropServices;
namespace Ngs.Interop {
    /// <summary>
    /// Provides a collection of method for interacting unmanaged code based on
    /// the NgsCOM ABI.
    /// </summary>
    public static partial class NgscomMarshal {
        /// <summary>
        /// Constructs a RCW (runtime-callable wrapper) for an NgsCOM interface
        /// pointer.
        /// </summary>
        /// <remarks>
        /// An RCW (runtime-callable wrapper) is a kind of an object that can be used to call the
        /// methods of an NgsPF interface from a managed code. An RCW is created from an interface
        /// pointer, and implements the interface and its all base interfaces. Note that an RCW does
        /// not support casting to other interfaces even if they are implemented by the object.
        /// You must use <see cref="QueryInterface" /> or <see cref="QueryInterfaceOrNull" /> to
        /// access them.
        /// </remarks>
        /// <param name="ptr">An NgsCOM interface pointer.</param>
        /// <param name="addRef">A flag indicating whether the reference count
        /// of the object must be incremented.</param>
        /// <typeparam name="T">An NgsCOM interface.</typeparam>
        /// <returns>The RCW object that can be used to call methods on <paramref name="ptr" />.
        /// </returns>
        [SecurityCritical]
        public static T GetRcwForInterfacePtr<T>(IntPtr ptr, bool addRef) where T : class, IUnknown {
            if (ptr == IntPtr.Zero) {
                return null;
            }
            return InterfaceRuntimeInfo<T>.CreateRcw(ptr, addRef);
        }

        /// <summary>
        /// Constructs a CCW (COM-callable wrapper) for an managed object, and
        /// returns an interface pointer (of type <typeparamref name="T" />) to
        /// it.
        /// </summary>
        /// <remarks>
        /// <para>A CCW (COM-callable wrapper) is a kind of an object that can be used to call the
        /// methods of a managed object implementing a certain NgsPF interface from a native code.
        /// </para>
        /// <para>If the supplied object originates from a native code (i.e. created from an
        /// interface pointer via <see cref="GetRcwForInterfacePtr" />), the generation of CCW is
        /// suppressed and the original interface pointer is returned instead.</para>
        /// </remarks>
        /// <param name="obj">The object to create a CCW from.</param>
        /// <typeparam name="T">An NgsCOM interface.</typeparam>
        /// <returns>An interface pointer (of type <typeparamref name="T" />).</returns>
        [SecurityCritical]
        public static IntPtr GetCcwForInterface<T>(T obj) where T : class, IUnknown {
            if (obj == null) {
                return IntPtr.Zero;
            }

            var nativeObj = obj as INativeObject<T>;
            if (nativeObj != null) {
                nativeObj.AddRef();
                return nativeObj.NativeInterfacePtr;
            }

            var guid = InterfaceRuntimeInfo<T>.ComGuid;
            return obj.QueryNativeInterface(ref guid);
        }

        /// <summary>
        /// Converts a given object to an NgsCOM interface. Returns <c>null</c> on failure.
        /// </summary>
        /// <remarks>
        /// This method is near equivalent to <c>obj as T</c>. However, this method implements
        /// the conversion of an RCW (runtime-callable wrapper) correctly.
        /// </remarks>
        /// <param name="obj">The object being converted.</param>
        /// <typeparam name="T">An NgsCOM interface.</typeparam>
        /// <returns>The converted object.</returns>
        [SecuritySafeCritical]
        public static T QueryInterfaceOrNull<T>(IUnknown obj) where T : class, IUnknown {
            var casted = obj as T;
            if (casted == null) {
                const int E_NOINTERFACE = unchecked((int)0x80004002);
                try {
                    var guid = InterfaceRuntimeInfo<T>.ComGuid;
                    var ptr = obj.QueryNativeInterface(ref guid);
                    return InterfaceRuntimeInfo<T>.CreateRcw(ptr, true);
                } catch (COMException ex) when (ex.HResult == E_NOINTERFACE) {
                    return null;
                }
            }
            return casted;
        }

        /// <summary>
        /// Converts a given object to an NgsCOM interface.
        /// </summary>
        /// <remarks>
        /// This method is near equivalent to <c>(T)obj</c>. However, this method implements
        /// the conversion of an RCW (runtime-callable wrapper) correctly.
        /// </remarks>
        /// <param name="obj">The object being converted.</param>
        /// <typeparam name="T">An NgsCOM interface.</typeparam>
        /// <returns>The converted object.</returns>
        [SecuritySafeCritical]
        public static T QueryInterface<T>(IUnknown obj) where T : class, IUnknown {
            if (!(obj is T)) {
                var guid = InterfaceRuntimeInfo<T>.ComGuid;
                var ptr = obj.QueryNativeInterface(ref guid);
                return InterfaceRuntimeInfo<T>.CreateRcw(ptr, true);
            }
            return (T)obj;
        }
    }
}