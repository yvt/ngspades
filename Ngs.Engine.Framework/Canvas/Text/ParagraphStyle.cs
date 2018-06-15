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
    /// A mutable set of paragraph styles.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFParagraphStyle" />.</para>
    /// </remarks>
    public class ParagraphStyle : ReadOnlyParagraphStyle {
        private CharacterStyle characterStyle;

        /// <summary>
        /// Initializes a new instance of <see cref="ParagraphStyle" />.
        /// </summary>
        public ParagraphStyle() :
            this(EngineInstance.NativeEngine.FontFactory.CreateParagraphStyle()) { }

        internal ParagraphStyle(INgsPFParagraphStyle nativeObject) : base(nativeObject) {
            characterStyle = new CharacterStyle(nativeObject.CharStyle);
        }

        /// <summary>
        /// Copies properties from a supplied <see cref="ReadOnlyParagraphStyle" /> instance.
        /// </summary>
        /// <param name="from">The <see cref="ReadOnlyParagraphStyle" /> instance to copy property
        /// values from.</param>
        public void CopyFrom(ReadOnlyParagraphStyle from) {
            if (from == null) {
                throw new ArgumentNullException(nameof(from));
            }

            MinimumLineHeight = from.MinimumLineHeight;
            LineHeightFactor = from.LineHeightFactor;
            TextAlign = from.TextAlign;
            TextDirection = from.TextDirection;
            WordWrapMode = from.WordWrapMode;
            CharacterStyle.CopyFrom(from.CharacterStyle);
        }

        /// <summary>
        /// Creates a read-only wrapper for this object.
        /// </summary>
        /// <remarks>
        /// The returned read-only wrapper reflects the changes made to the original
        /// <see cref="ParagraphStyle" />.
        /// </remarks>
        /// <returns>A read-only wrapper for this object.</returns>
        public ReadOnlyParagraphStyle AsReadOnly() => new ReadOnlyParagraphStyle(NativeParagraphStyle);

        public new float MinimumLineHeight {
            get => NativeParagraphStyle.MinimumLineHeight;
            set => NativeParagraphStyle.MinimumLineHeight = value;
        }

        public new float LineHeightFactor {
            get => NativeParagraphStyle.LineHeightFactor;
            set => NativeParagraphStyle.LineHeightFactor = value;
        }

        public new TextAlign TextAlign {
            get => NativeParagraphStyle.TextAlign;
            set => NativeParagraphStyle.TextAlign = value;
        }

        public new TextDirection TextDirection {
            get => NativeParagraphStyle.TextDirection;
            set => NativeParagraphStyle.TextDirection = value;
        }

        public new WordWrapMode WordWrapMode {
            get => NativeParagraphStyle.WordWrapMode;
            set => NativeParagraphStyle.WordWrapMode = value;
        }

        /// <summary>
        /// Sets or retrieves the default character style.
        /// </summary>
        /// <remarks>
        /// This property returns a <see cref="CharacterStyle" /> instance that can be used to
        /// retrieve and modify the default character style stored within a paragraph style object.
        /// </remarks>
        /// <returns>The default character style.</returns>
        public new CharacterStyle CharacterStyle {
            get => characterStyle;
        }
    }
}
