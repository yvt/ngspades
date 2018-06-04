//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Engine;
using Ngs.Engine.Canvas.Text;

namespace Ngs.Engine.Native {
    /// <summary>
    /// A set of character styles.
    /// </summary>
    [Guid("ac4130f1-bd7a-4432-92a1-7b8d67bc857e")]
    public interface INgsPFCharStyle : IUnknown {
        /// <summary>
        /// Sets of retrieves the comma-separated list of font family names.
        /// </summary>
        /// <returns>The comma-separated list of font family names.</returns>
        string FontFamilies { get; set; }

        /// <summary>
        /// Sets or retrieves the desired weight of the font.
        /// </summary>
        /// <returns>The weight of the font. Must be in range <c>[1, 1000]</c>. <c>0</c> indicates
        /// the inherited value.</returns>
        int FontWeight { get; set; }

        /// <summary>
        /// Sets or retrieves the style of the font.
        /// </summary>
        /// <returns>The style of the font.</returns>
        FontStyle FontStyle { get; set; }

        /// <summary>
        /// Sets or retrieves flags specifying the appearance of decorative lines used on the text.
        /// </summary>
        /// <returns></returns>
        TextDecoration TextDecoration { get; set; }

        /// <summary>
        /// Sets or retrieves the font size.
        /// </summary>
        /// <returns>The font size. <c>NaN</c> indicates the inherited value.</returns>
        double FontSize { get; set; }

        /// <summary>
        /// Sets or retrieves the color of the text.
        /// </summary>
        /// <returns>The color of the text. <c>NaN</c> indicates the inherited value.</returns>
        Rgba Color { get; set; }

        /// <summary>
        /// Sets or retrieves the language of the text.
        /// </summary>
        /// <returns>The language object. <c>null</c> indicates the default or inherited value.</returns>
        IUnknown Language { get; set; }

        /// <summary>
        /// Sets or retrieves the script of the text.
        /// </summary>
        /// <returns>The script object. <c>null</c> indicates the default or inherited value.</returns>
        IUnknown Script { get; set; }
    }
}