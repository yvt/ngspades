//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Native;

namespace Ngs.Engine.Canvas.Text {
    /// <summary>
    /// A set of paragraph styles.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFParagraphStyle" />.</para>
    /// </remarks>
    public class ParagraphStyle {
        private INgsPFParagraphStyle nativeObject;
        private CharacterStyle characterStyle;

        internal ParagraphStyle(INgsPFParagraphStyle nativeObject) {
            this.nativeObject = nativeObject;
            this.characterStyle = new CharacterStyle(nativeObject.CharStyle);
        }

        /// <summary>
        /// Initializes a new instance of <see cref="ParagraphStyle" />.
        /// </summary>
        public ParagraphStyle() :
            this(EngineInstance.NativeEngine.FontFactory.CreateParagraphStyle()) { }

        /// <summary>
        /// Creates a shallow copy of an instance of this class.
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
            set => nativeObject.MinimumLineHeight = value;
        }

        public float LineHeightFactor {
            get => nativeObject.LineHeightFactor;
            set => nativeObject.LineHeightFactor = value;
        }

        public TextAlign TextAlign {
            get => nativeObject.TextAlign;
            set => nativeObject.TextAlign = value;
        }

        public TextDirection TextDirection {
            get => nativeObject.TextDirection;
            set => nativeObject.TextDirection = value;
        }

        public WordWrapMode WordWrapMode {
            get => nativeObject.WordWrapMode;
            set => nativeObject.WordWrapMode = value;
        }

        /// <summary>
        /// Sets or retrieves the default character style.
        /// </summary>
        /// <remarks>
        /// This property returns a `CharacterStyle` instance that can be used to access the default
        /// character style stored within a paragraph style object.
        /// </remarks>
        /// <returns>The default character style.</returns>
        public CharacterStyle CharacterStyle {
            get => characterStyle;
        }
    }
}
