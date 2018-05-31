//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.UI {
    /// <summary>
    /// Describes how to align a layout element within its containing box.
    /// </summary>
    [Flags]
    public enum Alignment : byte {
        /// Specifies to align an element with the left edge.
        Left = 0x1,
        /// Specifies to align an element with the right edge.
        Right = 0x2,
        /// Specifies to center an element horizontically.
        HorizontalCenter = 0,
        /// Specifies to justify an element horizontically.
        HorizontalJustify = Left | Right,

        /// The mask for the horizontal alignemnt flags.
        HorizontalMask = 0x3,

        /// Specifies to align an element with the top edge.
        Top = 0x10,
        /// Specifies to align an element with the bottom edge.
        Bottom = 0x20,
        /// Specifies to center an element vertically.
        VerticalCenter = 0,
        /// Specifies to justify an element vertically.
        VerticalJustify = Top | Bottom,

        /// The mask for the vertical alignemnt flags.
        VerticalMask = 0x30,

        /// Specifies to center an element.
        Center = HorizontalCenter | VerticalCenter,
        /// Specifies to justify an element.
        Justify = HorizontalJustify | VerticalJustify,
    }
}