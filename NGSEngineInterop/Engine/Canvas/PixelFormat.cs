//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Canvas {
    /// <summary>
    /// Specifies the representation format of pixels in a bitmap.
    /// </summary>
    public enum PixelFormat {
        /// <summary>
        /// Represents a pixel format with a 8-bit red/green/blue/alpha channels in
        /// the sRGB encoding and in BGRA order.
        /// The alpha value is not premultiplied.
        /// </summary>
        SrgbRgba8,
        /// <summary>
        /// Represents a pixel format with a 8-bit red/green/blue/alpha channels in
        /// the sRGB encoding and in BGRA order.
        /// The alpha value is premultiplied.
        /// </summary>
        SrgbRgba8Premul,
    }
}