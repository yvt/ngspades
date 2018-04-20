//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Presentation {
    /// <summary>
    /// An interface to the window system of the target system.
    /// </summary>
    [Guid("605d9976-ab88-47cf-b68b-e1c2dfeaaa99")]
    public interface IWorkspace : IUnknown {
        /// <summary>
        /// Retrieves the associated presentation context.
        /// </summary>
        /// <returns>The presentation context managed by this workspace.</returns>
        IPresentationContext Context { get; }

        /// <summary>
        /// Sets the window node.
        /// </summary>
        /// <returns>
        /// The top-level window node (or a group of window nodes) of the workspace, or the <c>null</c> value.
        /// </returns>
        IUnknown Windows { set; }

        /// <summary>
        /// Enter the main loop.
        /// </summary>
        /// <remarks>
        /// This method returns only if an exit request was made from another thread by calling
        /// the <see cref="Exit" /> method.
        /// </remarks>
        void Start();

        /// <summary>
        /// Causes the main loop to terminate.
        /// </summary>
        /// <remarks>
        /// The exit request is a part of presentation properties. You must call
        /// <see cref="IPresentationContext.CommitFrame" /> afterward for it to take effect.
        /// </remarks>
        void Exit();
    }
}