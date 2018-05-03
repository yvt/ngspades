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
    /// Specifies the writing direction of a text.
    /// </summary>
    public enum TextDirection {
        /// <summary>
        /// Specifies the left-to-right writing direction.
        /// </summary>
        LeftToRight,
        /// <summary>
        /// Specifies the right-to-left writing direction.
        /// </summary>
        RightToLeft,
        /// <summary>
        /// Specifies the top-to-bottom writing direction.
        /// </summary>
        TopToBottom,
        /// <summary>
        /// Specifies the bottom-to-top writing direction.
        /// </summary>
        BottomToTop,
    }
}