//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.Canvas.Text {
    /// <summary>
    /// Specifies the appearance of decorative lines used on the text.
    /// </summary>
    [Flags]
    public enum TextDecoration {
        /// <summary>
        /// Specifies that no decorative lines are drawn.
        /// </summary>
        None = 0,
        /// <summary>
        /// Specifies that a decorative line is drawn beneath each line of text.
        /// </summary>
        Underline = 1 << 0,
        /// <summary>
        /// Specifies that a decorative line is drawn above each line of text.
        /// </summary>
        Overline = 1 << 1,
        /// <summary>
        /// Specifies that a decorative line is drawn on each line of text.
        /// </summary>
        Strikethrough = 1 << 2,

        /// <summary>
        /// Inherits the parent element's style.
        /// </summary>
        Inherited = 1 << 3,
    }
}