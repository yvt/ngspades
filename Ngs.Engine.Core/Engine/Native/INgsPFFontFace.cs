//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Engine;

namespace Ngs.Engine.Native {
    /// <summary>
    /// Represents a single font face in a font file.
    /// </summary>
    [Guid("49e30a07-f8bd-46b4-b5f8-0d7853c963f0")]
    public interface INgsPFFontFace : IUnknown {
    }
}