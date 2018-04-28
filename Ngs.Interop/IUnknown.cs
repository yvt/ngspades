//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using System.Security;

namespace Ngs.Interop {
    /// <summary>
    /// Provides the base functionality of an NgsCOM object. Enables clients to get interface
    /// pointers for interfaces implemented by a given object. Implements reference counting.
    /// </summary>
    [Guid("00000000-0000-0000-C000-000000000046")]
    public interface IUnknown {
        /// <summary>
        /// Retrieves an interface pointer for the interface implemented by an object. Fails with
        /// <c>E_NOINTERFACE</c> if the interface is not implemented.
        /// </summary>
        /// <param name="guid">The GUID of the requested interface.</param>
        /// <returns>The interface pointer.</returns>
        [SecurityCritical]
        IntPtr QueryNativeInterface([In] ref Guid guid);

        /// <summary>
        /// Increments the reference count.
        /// </summary>
        /// <returns>The new reference count. This value is intended to be used only for debugging
        /// purposes.</returns>
        [PreserveSig, SecurityCritical]
        uint AddRef();

        /// <summary>
        /// Decrements the reference count.
        /// </summary>
        /// <returns>The new reference count. This value is intended to be used only for debugging
        /// purposes.</returns>
        [PreserveSig, SecurityCritical]
        uint Release();
    }
}