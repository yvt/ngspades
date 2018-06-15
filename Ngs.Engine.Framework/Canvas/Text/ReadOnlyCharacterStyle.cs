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
    /// An immutable set of character styles.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFCharStyle" />.</para>
    /// </remarks>
    public class ReadOnlyCharacterStyle {
        private INgsPFCharStyle nativeObject;

        internal ReadOnlyCharacterStyle(INgsPFCharStyle nativeObject) {
            this.nativeObject = nativeObject;
        }

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
        /// Retrieves the list of font family names.
        /// </summary>
        /// <returns>The font family names.</returns>
        public string[] GetFontFamilies() => nativeObject.FontFamilies.Split(',');

        /// <summary>
        /// Retrieves the desired weight of the font.
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
        }

        /// <summary>
        /// Retrieves the style of the font.
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
        }

        /// <summary>
        /// Retrieves flags specifying the appearance of decorative lines used on the text.
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
        }

        /// <summary>
        /// Retrieves the font size.
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
        }

        /// <summary>
        /// Retrieves the color of the text.
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
        }

        /// <summary>
        /// Retrieves the language of the text.
        /// </summary>
        /// <returns>The language object. <c>null</c> indicates the default or inherited value.</returns>
        public Language? Language {
            get {
                var x = nativeObject.Language;
                return x != null ? new Language(x) : (Language?)null;
            }
        }

        /// <summary>
        /// Retrieves the script of the text.
        /// </summary>
        /// <returns>The script object. <c>null</c> indicates the default or inherited value.</returns>
        public Script? Script {
            get {
                var x = nativeObject.Script;
                return x != null ? new Script(x) : (Script?)null;
            }
        }
    }
}
