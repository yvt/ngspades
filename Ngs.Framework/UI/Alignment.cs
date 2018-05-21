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
        HorizontalCenter = 0x4,
        /// Specifies to justify an element horizontically.
        HorizontalJustify = 0x8,

        /// The mask for the horizontal alignemnt flags.
        HorizontalMask = 0xf,

        /// Specifies to align an element with the top edge.
        Top = 0x10,
        /// Specifies to align an element with the bottom edge.
        Bottom = 0x20,
        /// Specifies to center an element vertically.
        VerticalCenter = 0x40,
        /// Specifies to justify an element vertically.
        VerticalJustify = 0x40,

        /// The mask for the vertical alignemnt flags.
        VerticalMask = 0x70,

        /// Specifies to center an element.
        Center = HorizontalCenter | VerticalCenter,
        /// Specifies to justify an element.
        Justify = HorizontalJustify | VerticalJustify,
    }
}