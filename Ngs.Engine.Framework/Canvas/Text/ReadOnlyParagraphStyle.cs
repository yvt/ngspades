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
    /// An immutable set of paragraph styles.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFParagraphStyle" />.</para>
    /// </remarks>
    public class ReadOnlyParagraphStyle {
        private INgsPFParagraphStyle nativeObject;
        private ReadOnlyCharacterStyle characterStyle;

        internal ReadOnlyParagraphStyle(INgsPFParagraphStyle nativeObject) {
            this.nativeObject = nativeObject;
            this.characterStyle = new ReadOnlyCharacterStyle(nativeObject.CharStyle);
        }

        /// <summary>
        /// Creates a copy of an instance of this class.
        /// </summary>
        /// <returns>A newly created instance of this class.</returns>
        public ParagraphStyle Clone() {
            var ret = new ParagraphStyle()
            {
                MinimumLineHeight = MinimumLineHeight,
                LineHeightFactor = LineHeightFactor,
                TextAlign = TextAlign,
                TextDirection = TextDirection,
                WordWrapMode = WordWrapMode,
            };
            ret.CharacterStyle.CopyFrom(CharacterStyle);
            return ret;
        }

        internal INgsPFParagraphStyle NativeParagraphStyle {
            [SecurityCritical]
            get => nativeObject;
        }

        public float MinimumLineHeight {
            get => nativeObject.MinimumLineHeight;
        }

        public float LineHeightFactor {
            get => nativeObject.LineHeightFactor;
        }

        public TextAlign TextAlign {
            get => nativeObject.TextAlign;
        }

        public TextDirection TextDirection {
            get => nativeObject.TextDirection;
        }

        public WordWrapMode WordWrapMode {
            get => nativeObject.WordWrapMode;
        }

        /// <summary>
        /// Sets or retrieves the default character style.
        /// </summary>
        /// <remarks>
        /// This property returns a <see cref="ReadOnlyCharacterStyle" /> instance that can be used
        /// to access the default character style stored within a paragraph style object.
        /// </remarks>
        /// <returns>The default character style.</returns>
        public ReadOnlyCharacterStyle CharacterStyle {
            get => characterStyle;
        }
    }
}
