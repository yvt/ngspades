//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.Presentation {
    /// <summary>
    /// Specifies properties of a window.
    /// </summary>
    [Flags]
    public enum WindowFlags {
        /// <summary>
        /// Specifies that the window can be resized by the user.
        /// </summary>
        Resizable = 1 << 0,

        /// <summary>
        /// Hides the window's decoration (title bar, border, etc.).
        /// </summary>
        Borderless = 1 << 1,

        /// <summary>
        /// Makes the background of the window transparent.
        /// </summary>
        Transparent = 1 << 2,
    }
}