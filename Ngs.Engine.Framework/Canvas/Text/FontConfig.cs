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
    /// Maintains a mutable set of fonts and their associated font style values which are
    /// used to determine the optimal font for rendering given characters.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFFontConfig" />.</para>
    /// </remarks>
    public class FontConfig : ReadOnlyFontConfig {
        /// <summary>
        /// Initializes a new instance of <see cref="FontConfig" />.
        /// </summary>
        public FontConfig() :
            base(EngineInstance.NativeEngine.FontFactory.CreateFontConfig()) { }

        /// <summary>
        /// Adds a font face into the font search list.
        /// </summary>
        /// <param name="fontFace">The font face to be added.</param>
        /// <param name="fontFamily">The font family name associated with the font face.</param>
        /// <param name="fontStyle">The font style associated with the font face. Mustn't be <c>Inherited</c>.</param>
        /// <param name="weight">The font weight associated with the font face. Must be in range <c>[100, 1000]</c>.</param>
        [SecuritySafeCritical]
        public void AddFontFace(FontFace fontFace, string fontFamily, FontStyle fontStyle, int weight) {
            if (fontFamily == null) {
                throw new ArgumentNullException(nameof(fontFamily));
            }
            if (fontFace.NativeFontFace == null) {
                throw new ArgumentException(nameof(fontFamily));
            }
            if (weight < 100 || weight > 1000) {
                throw new ArgumentOutOfRangeException(nameof(weight));
            }

            NativeFontConfig.AddFontFace(fontFace.NativeFontFace, fontFamily, fontStyle, weight);
        }

        /// <summary>
        /// Creates a read-only wrapper for this font config.
        /// </summary>
        /// <remarks>
        /// The returned read-only wrapper reflects the changes made to the original
        /// <see cref="FontConfig" />.
        /// </remarks>
        /// <returns>A read-only wrapper for this font config.</returns>
        public ReadOnlyFontConfig AsReadOnly() => new ReadOnlyFontConfig(NativeFontConfig);
    }
}
