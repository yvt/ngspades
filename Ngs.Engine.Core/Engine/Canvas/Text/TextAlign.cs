//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Canvas.Text {
    /// <summary>
    /// Specifies the alignment of each line in text.
    /// </summary>
    public enum TextAlign {
        /// <summary>
        /// Aligns each line toward the opposite of the writing direction.
        /// </summary>
        Start,
        /// <summary>
        /// Aligns each line toward the writing direction.
        /// </summary>
        End,
        /// <summary>
        /// Aligns each line to the center of the containing box.
        /// </summary>
        Center,

        /// <summary>
        /// Justifies every line except the last one.
        /// </summary>
        /// <remarks>
        /// This value is invalid for point type and interpreted as `Start`.
        /// </remarks>
        Justify,

        /// <summary>
        /// Justifies every line.
        /// </summary>
        /// <remarks>
        /// This value is invalid for point type and interpreted as `Start`.
        /// </remarks>
        JustifyAll,
    }
}