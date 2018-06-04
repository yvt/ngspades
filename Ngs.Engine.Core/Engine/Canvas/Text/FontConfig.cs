//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Engine;
using Ngs.Engine.Native;

namespace Ngs.Engine.Canvas.Text {
    /// <summary>
    /// Maintains a set of fonts and their associated font style values which are
    /// used to determine the optimal font for rendering given characters.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFFontConfig" />.</para>
    /// </remarks>
    public class FontConfig {
        private INgsPFFontConfig nativeObject;

        internal FontConfig(INgsPFFontConfig nativeObject) {
            this.nativeObject = nativeObject;
        }

        /// <summary>
        /// Initializes a new instance of <see cref="FontConfig" />.
        /// </summary>
        public FontConfig() :
            this(EngineInstance.NativeEngine.FontFactory.CreateFontConfig()) { }

        internal INgsPFFontConfig NativeFontConfig {
            [SecurityCritical]
            get => nativeObject;
        }

        /// <summary>
        /// Adds a font face into the font search list.
        /// </summary>
        /// <param name="fontFace">The font face to be added.</param>
        /// <param name="fontFamily">The font family name associated with the font face.</param>
        /// <param name="fontStyle">The font style associated with the font face. Mustn't be <c>Inherited</c>.</param>
        /// <param name="weight">The font weight associated with the font face. Must be in range <c>[100, 1000]</c>.</param>
        [SecuritySafeCritical]
        public void AddFontFace(FontFace fontFace, string fontFamily, FontStyle fontStyle, int weight) {
            if (weight < 100 || weight > 1000) {
                throw new ArgumentOutOfRangeException(nameof(weight));
            }
            nativeObject.AddFontFace(fontFace.NativeFontFace, fontFamily, fontStyle, weight);
        }

        /// <summary>
        /// Layouts a given string as point type.
        /// </summary>
        /// <param name="text">The text to be layouted.</param>
        /// <param name="paragraphStyle">The paragraph style.</param>
        /// <returns>The layouted text.</returns>
        [SecuritySafeCritical]
        public TextLayout LayoutString(string text, ParagraphStyle paragraphStyle) {
            return new TextLayout(nativeObject.LayoutPointString(text,
                paragraphStyle.NativeParagraphStyle));
        }

        /// <summary>
        /// Layouts a given string as area type.
        /// </summary>
        /// <param name="text">The text to be layouted.</param>
        /// <param name="paragraphStyle">The paragraph style.</param>
        /// <param name="width">The width of the bounding box. The axis used for this parameter is
        /// dependent on the value of the <see cref="ParagraphStyle.TextDirection" /> property.</param>
        /// <returns>The layouted text.</returns>
        [SecuritySafeCritical]
        public TextLayout LayoutString(string text, ParagraphStyle paragraphStyle, float width) {
            return new TextLayout(nativeObject.LayoutBoxAreaString(text,
                paragraphStyle.NativeParagraphStyle, width));
        }

    }
}