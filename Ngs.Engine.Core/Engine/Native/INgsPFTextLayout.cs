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
    /// A layouted text.
    /// </summary>
    [Guid("869dbfb8-cf34-4d2f-9339-98b0ba422a02")]
    public interface INgsPFTextLayout : IUnknown {
        /// <summary>
        /// Computes and retrieves the visual bounding rectangle of the layouted text.
        /// </summary>
        /// <returns>The visual bounding rectangle of the layouted text.</returns>
        Box2 VisualBounds { get; }
    }
}