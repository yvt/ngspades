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
    /// Maintains an immutable set of fonts and their associated font style values which are
    /// used to determine the optimal font for rendering given characters.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFFontConfig" />.</para>
    /// </remarks>
    public class ReadOnlyFontConfig {
        private INgsPFFontConfig nativeObject;

        internal ReadOnlyFontConfig(INgsPFFontConfig nativeObject) {
            this.nativeObject = nativeObject;
        }

        internal INgsPFFontConfig NativeFontConfig {
            [SecurityCritical]
            get => nativeObject;
        }

        /// <summary>
        /// Layouts a given string as point type.
        /// </summary>
        /// <param name="text">The text to be layouted.</param>
        /// <param name="paragraphStyle">The paragraph style.</param>
        /// <returns>The layouted text.</returns>
        [SecuritySafeCritical]
        public TextLayout LayoutString(string text, ReadOnlyParagraphStyle paragraphStyle) {
            if (text == null) {
                throw new ArgumentNullException(nameof(text));
            }
            if (paragraphStyle == null) {
                throw new ArgumentNullException(nameof(paragraphStyle));
            }

            return new TextLayout(nativeObject.LayoutPointString(text,
                paragraphStyle.NativeParagraphStyle));
        }

        /// <summary>
        /// Layouts a given string as area type.
        /// </summary>
        /// <param name="text">The text to be layouted.</param>
        /// <param name="paragraphStyle">The paragraph style.</param>
        /// <param name="width">The width of the bounding box. The axis used for this parameter is
        /// dependent on the value of the <see cref="ReadOnlyParagraphStyle.TextDirection" />
        /// roperty.</param>
        /// <returns>The layouted text.</returns>
        [SecuritySafeCritical]
        public TextLayout LayoutString(string text, ReadOnlyParagraphStyle paragraphStyle, float width) {
            if (text == null) {
                throw new ArgumentNullException(nameof(text));
            }
            if (paragraphStyle == null) {
                throw new ArgumentNullException(nameof(paragraphStyle));
            }

            return new TextLayout(nativeObject.LayoutBoxAreaString(text,
                paragraphStyle.NativeParagraphStyle, width));
        }

    }
}