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
        /// <para>
        /// Specifies that the window can be resized by the user.
        /// </para>
        /// <para>
        /// In the current implementation of NgsPF, fixed-sized windows do not work well.
        /// </para>
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

        /// <summary>
        /// <para>
        /// Disables the default behavior of the window's close button.
        /// </para>
        /// <para>
        /// This flag is currently not implemented by NgsPF.
        /// </para>
        /// </summary>
        DenyUserClose = 1 << 3,
    }
}