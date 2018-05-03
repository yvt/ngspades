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
    /// Specifies a word-wrapping algorithm.
    /// </summary>
    public enum WordWrapMode {
        /// <summary>
        /// Minimizes the number of lines. This mode is commonly used by most
        /// operating systems and word processors.
        /// </summary>
        MinNumLines,

        /// <summary>
        /// Minimizes the raggedness. This mode is often used by high-quality
        /// typesetting systems such as Adobe InDesign and LaTeX.
        /// </summary>
        MinRaggedness,
    }
}