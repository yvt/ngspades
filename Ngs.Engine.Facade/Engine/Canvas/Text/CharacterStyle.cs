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
    /// A set of character styles.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFCharStyle" />.</para>
    /// </remarks>
    public class CharacterStyle {
        private INgsPFCharStyle nativeObject;

        internal CharacterStyle(INgsPFCharStyle nativeObject) {
            this.nativeObject = nativeObject;
        }

        /// <summary>
        /// Initializes a new instance of <see cref="CharacterStyle" />.
        /// </summary>
        public CharacterStyle() :
            this(EngineInstance.NativeEngine.FontFactory.CreateCharStyle()) { }

        internal INgsPFCharStyle NativeCharStyle {
            [SecurityCritical]
            get => nativeObject;
        }

        /// <summary>
        /// Creates a shallow copy of an instance of this class.
        /// </summary>
        /// <returns>A newly created instance of this class.</returns>
        [SecuritySafeCritical]
        public CharacterStyle Clone() {
            var newObj = new CharacterStyle()
            {
                FontWeight = FontWeight,
                FontStyle = FontStyle,
                FontSize = FontSize,
                Color = Color,
                Language = Language,
                Script = Script,
            };
            newObj.NativeCharStyle.FontFamilies = NativeCharStyle.FontFamilies;
            return newObj;
        }

        /// <summary>
        /// Copies properties from another instance of this class.
        /// </summary>
        /// <param name="from">The instance to copy property values from.</param>
        [SecuritySafeCritical]
        public void CopyFrom(CharacterStyle from) {
            FontWeight = from.FontWeight;
            FontStyle = from.FontStyle;
            FontSize = from.FontSize;
            Color = from.Color;
            Language = from.Language;
            Script = from.Script;
            nativeObject.FontFamilies = from.nativeObject.FontFamilies;
        }

        /// <summary>
        /// Retrieves the comma-separated list of font family names.
        /// </summary>
        /// <returns>The font family names.</returns>
        public string[] GetFontFamilies() => nativeObject.FontFamilies.Split(',');

        /// <summary>
        /// Sets the comma-separated list of font family names.
        /// </summary>
        /// <param name="fontFamilies">The font family names. May not contain a comma.</param>
        public void SetFontFamilies(string[] fontFamilies) =>
            nativeObject.FontFamilies = fontFamilies != null ?
                string.Join(',', fontFamilies) : null;

        /// <summary>
        /// Sets or retrieves the desired weight of the font.
        /// </summary>
        /// <returns>The weight of the font. Must be in range <c>[100, 1000]</c>.
        /// <c>null</c> indicates the inherited value.</returns>
        public int? FontWeight {
            get {
                var x = nativeObject.FontWeight;
                if (x == 0) {
                    return null;
                } else {
                    return x;
                }
            }
            set {
                if (value.HasValue && (value < 100 || value > 1000)) {
                    throw new ArgumentOutOfRangeException(nameof(value));
                }
                nativeObject.FontWeight = value ?? 0;
            }
        }

        /// <summary>
        /// Sets or retrieves the style of the font.
        /// </summary>
        /// <returns>The style of the font.</returns>
        public FontStyle? FontStyle {
            get {
                var x = nativeObject.FontStyle;
                if (x == Text.FontStyle.Inherited) {
                    return null;
                } else {
                    return x;
                }
            }
            set => nativeObject.FontStyle = value ?? Text.FontStyle.Inherited;
        }

        /// <summary>
        /// Sets or retrieves flags specifying the appearance of decorative lines used on the text.
        /// </summary>
        /// <returns>The text decoration flags. <c>null</c> indicates the inherited value.</returns>
        public TextDecoration? TextDecoration {
            get {
                var x = nativeObject.TextDecoration;
                if ((x & Text.TextDecoration.Inherited) != Text.TextDecoration.None) {
                    return null;
                } else {
                    return x;
                }
            }
            set => nativeObject.TextDecoration = value ?? Text.TextDecoration.Inherited;
        }

        /// <summary>
        /// Sets or retrieves the font size.
        /// </summary>
        /// <returns>The font size. <c>null</c> indicates the inherited value.</returns>
        public double? FontSize {
            get {
                var x = nativeObject.FontSize;
                if (double.IsNaN(x)) {
                    return null;
                } else {
                    return x;
                }
            }
            set => nativeObject.FontSize = value ?? double.NaN;
        }

        /// <summary>
        /// Sets or retrieves the color of the text.
        /// </summary>
        /// <returns>The color of the text. <c>null</c> indicates the inherited value.</returns>
        public Rgba? Color {
            get {
                var x = nativeObject.Color;
                if (float.IsNaN(x.Alpha)) {
                    return null;
                } else {
                    return x;
                }
            }
            set => nativeObject.Color = value ?? new Rgba(float.NaN, float.NaN, float.NaN, float.NaN);
        }

        /// <summary>
        /// Sets or retrieves the language of the text.
        /// </summary>
        /// <returns>The language object. <c>null</c> indicates the default or inherited value.</returns>
        public Language? Language {
            get {
                var x = nativeObject.Language;
                return x != null ? new Language(x) : (Language?)null;
            }
            [SecuritySafeCritical]
            set => nativeObject.Language = value?.NativeObject ?? null;
        }

        /// <summary>
        /// Sets or retrieves the script of the text.
        /// </summary>
        /// <returns>The script object. <c>null</c> indicates the default or inherited value.</returns>
        public Script? Script {
            get {
                var x = nativeObject.Script;
                return x != null ? new Script(x) : (Script?)null;
            }
            [SecuritySafeCritical]
            set => nativeObject.Script = value?.NativeObject ?? null;
        }
    }
}
