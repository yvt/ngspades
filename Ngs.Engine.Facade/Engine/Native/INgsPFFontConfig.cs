//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Canvas.Text;

namespace Ngs.Engine.Native {
    /// <summary>
    /// Maintains a set of fonts and their associated font style values which are
    /// used to determine the optimal font for rendering given characters.
    /// </summary>
    [Guid("cd0f6c8d-9ab8-4f7e-9050-ec9b1a4e0bc1")]
    public interface INgsPFFontConfig : IUnknown {
        /// <summary>
        /// Adds a font face into the font search list.
        /// </summary>
        /// <param name="fontFace">The font face to be added.</param>
        /// <param name="fontFamily">The font family name associated with the font face.</param>
        /// <param name="fontStyle">The font style associated with the font face. Mustn't be <c>Inherited<c/>.</param>
        /// <param name="weight">The font weight associated with the font face.</param>
        void AddFontFace(INgsPFFontFace fontFace, string fontFamily, FontStyle fontStyle, int weight);

        INgsPFTextLayout LayoutPointString(string text, INgsPFParagraphStyle paragraphStyle);

        INgsPFTextLayout LayoutBoxAreaString(string text, INgsPFParagraphStyle paragraphStyle, float width);

        // TODO: Rich text
    }
}