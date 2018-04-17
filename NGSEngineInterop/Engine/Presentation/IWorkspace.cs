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
        IPresentationContext Context { get; }

        void CommitFrame ();
    }
}