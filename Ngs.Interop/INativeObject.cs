//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;

namespace Ngs.Interop {
    /// <summary>
    /// NgsCOM infrastructure. Not intended to be used by an application.
    /// </summary>
    /// <remarks>
    /// Indicates an object that is a RCW (runtime-callable wrapper) created from an interface
    /// pointer of type <typeparamref name="T" />.
    /// </remarks>
    /// <typeparam name="T">An NgsCOM interface.</typeparam>
    public interface INativeObject<out T> : IUnknown where T : class, IUnknown {
        /// <summary>
        /// NgsCOM infrastructure. Not intended to be used by an application.
        /// </summary>
        /// <remarks>
        /// The interface pointer of type <typeparamref name="T" />.
        /// </remarks>
        IntPtr NativeInterfacePtr { [SecurityCritical] get; }
        /// <summary>
        /// NgsCOM infrastructure. Not intended to be used by an application.
        /// </summary>
        /// <remarks>
        /// The interface pointer of type <c>IUnknown</c>.
        /// </remarks>
        IntPtr NativeIUnknownPtr { [SecurityCritical] get; }

        /// <summary>
        /// NgsCOM infrastructure. Not intended to be used by an application.
        /// </summary>
        /// <remarks>
        /// The interface object of type <typeparamref name="T" />.
        /// It's probably faster to use this property than performing a cast operation.
        /// </remarks>
        T Interface { get; }
    }
}