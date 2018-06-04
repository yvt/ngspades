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
    /// A font file loaded into the memory, containing one or more font faces.
    /// </summary>
    [Guid("890ec645-df1a-4989-a2f9-fa240fa3422d")]
    public interface INgsPFFont : IUnknown {
        /// <summary>
        /// Retrieves the number of font faces contained in this font.
        /// </summary>
        /// <returns>The number of font faces.</returns>
        int NumFontFaces { get; }

        /// <summary>
        /// Retrieves a font face contained in this font.
        /// </summary>
        /// <param name="index">The index of the font face. Must be in range
        /// <c>[0, NumFontFaces - 1]</c>.</param>
        /// <returns>A font face.</returns>
        INgsPFFontFace GetFontFace(int index);
    }
}