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
    /// Specifies a font style.
    /// </summary>
    public enum FontStyle {
        /// <summary>
        /// Inherits the parent element's style.
        /// </summary>
        Inherited,

        /// <summary>
        /// Specifies the normal style.
        /// </summary>
        Normal,

        /// <summary>
        /// Specifies the italic style.
        /// </summary>
        Italic,

        /// <summary>
        /// Specifies the oblique style.
        /// </summary>
        Oblique,
    }
}