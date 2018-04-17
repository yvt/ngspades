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
    [Guid ("605d9976-ab88-47cf-b68b-e1c2dfeaaa99")]
    public interface IWorkspace : IUnknown {
        /// <summary>
        /// Retrieves the associated presentation context.
        /// </summary>
        /// <returns>The presentation context managed by this workspace.</returns>
        IPresentationContext Context { get; }

        /// <summary>
        /// Captures the current state of presentation nodes and submit it for
        /// presentation.
        /// </summary>
        void CommitFrame ();
    }
}