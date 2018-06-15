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
    /// A mutable set of character styles.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFCharStyle" />.</para>
    /// </remarks>
    public class CharacterStyle : ReadOnlyCharacterStyle {
        /// <summary>
        /// Initializes a new instance of <see cref="CharacterStyle" />.
        /// </summary>
        public CharacterStyle() :
            base(EngineInstance.NativeEngine.FontFactory.CreateCharStyle()) { }

        internal CharacterStyle(INgsPFCharStyle nativeObject) : base(nativeObject) { }

        /// <summary>
        /// Creates a read-only wrapper for this object.
        /// </summary>
        /// <remarks>
        /// The returned read-only wrapper reflects the changes made to the original
        /// <see cref="CharacterStyle" />.
        /// </remarks>
        /// <returns>A read-only wrapper for this object.</returns>
        public ReadOnlyCharacterStyle AsReadOnly() => new ReadOnlyCharacterStyle(NativeCharStyle);

        /// <summary>
        /// Copies properties from a supplied <see cref="ReadOnlyCharacterStyle" /> instance.
        /// </summary>
        /// <param name="from">The <see cref="ReadOnlyCharacterStyle" /> instance to copy property
        /// values from.</param>
        [SecuritySafeCritical]
        public void CopyFrom(ReadOnlyCharacterStyle from) {
            FontWeight = from.FontWeight;
            FontStyle = from.FontStyle;
            FontSize = from.FontSize;
            Color = from.Color;
            Language = from.Language;
            Script = from.Script;
            NativeCharStyle.FontFamilies = from.NativeCharStyle.FontFamilies;
        }

        /// <summary>
        /// Sets the list of font family names.
        /// </summary>
        /// <param name="fontFamilies">The font family names. None of them may contain a comma.</param>
        public void SetFontFamilies(string[] fontFamilies) =>
            NativeCharStyle.FontFamilies = fontFamilies != null ?
                string.Join(',', fontFamilies) : null;

        /// <summary>
        /// Sets or retrieves the desired weight of the font.
        /// </summary>
        /// <returns>The weight of the font. Must be in range <c>[100, 1000]</c>.
        /// <c>null</c> indicates the inherited value.</returns>
        public new int? FontWeight {
            get => base.FontWeight;
            set {
                if (value.HasValue && (value < 100 || value > 1000)) {
                    throw new ArgumentOutOfRangeException(nameof(value));
                }
                NativeCharStyle.FontWeight = value ?? 0;
            }
        }

        /// <summary>
        /// Sets or retrieves the style of the font.
        /// </summary>
        /// <returns>The style of the font.</returns>
        public new FontStyle? FontStyle {
            get => base.FontStyle;
            set => NativeCharStyle.FontStyle = value ?? Text.FontStyle.Inherited;
        }

        /// <summary>
        /// Sets or retrieves flags specifying the appearance of decorative lines used on the text.
        /// </summary>
        /// <returns>The text decoration flags. <c>null</c> indicates the inherited value.</returns>
        public new TextDecoration? TextDecoration {
            get => base.TextDecoration;
            set => NativeCharStyle.TextDecoration = value ?? Text.TextDecoration.Inherited;
        }

        /// <summary>
        /// Sets or retrieves the font size.
        /// </summary>
        /// <returns>The font size. <c>null</c> indicates the inherited value.</returns>
        public new double? FontSize {
            get => base.FontSize;
            set => NativeCharStyle.FontSize = value ?? double.NaN;
        }

        /// <summary>
        /// Sets or retrieves the color of the text.
        /// </summary>
        /// <returns>The color of the text. <c>null</c> indicates the inherited value.</returns>
        public new Rgba? Color {
            get => base.Color;
            set => NativeCharStyle.Color = value ?? new Rgba(float.NaN, float.NaN, float.NaN, float.NaN);
        }

        /// <summary>
        /// Sets or retrieves the language of the text.
        /// </summary>
        /// <returns>The language object. <c>null</c> indicates the default or inherited value.</returns>
        public new Language? Language {
            get => base.Language;
            [SecuritySafeCritical]
            set => NativeCharStyle.Language = value?.NativeObject ?? null;
        }

        /// <summary>
        /// Sets or retrieves the script of the text.
        /// </summary>
        /// <returns>The script object. <c>null</c> indicates the default or inherited value.</returns>
        public new Script? Script {
            get => base.Script;
            [SecuritySafeCritical]
            set => NativeCharStyle.Script = value?.NativeObject ?? null;
        }
    }
}
