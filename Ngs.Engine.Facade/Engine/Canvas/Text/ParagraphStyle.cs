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

        internal ParagraphStyle(INgsPFParagraphStyle nativeObject) {
            this.nativeObject = nativeObject;
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
            return new ParagraphStyle()
            {
                MinimumLineHeight = MinimumLineHeight,
                LineHeightFactor = LineHeightFactor,
                TextAlign = TextAlign,
                TextDirection = TextDirection,
                WordWrapMode = WordWrapMode,
                CharStyle = CharStyle,
            };
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

        public CharacterStyle CharStyle {
            get => new CharacterStyle(nativeObject.CharStyle);
            set => nativeObject.CharStyle = value.NativeCharStyle;
        }
    }
}
