//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;

namespace Ngs.Engine.Canvas {
    /// <summary>
    /// Represents a mutable bitmap image.
    /// </summary>
    [Guid("d9cd03d0-3d98-481e-8668-e80e35b1f0b8")]
    public interface IBitmap : IUnknown {
        /// <summary>
        /// Retrieves the size of the bitmap.
        /// </summary>
        /// <returns>The size of the bitmap, measured in pixels.</returns>
        IntVector2 Size { get; }

        /// <summary>
        /// Retrieves the pixel format of the bitmap.
        /// </summary>
        /// <returns>The pixel format of the bitmap.</returns>
        PixelFormat Format { get; }

        /// <summary>
        /// Create a <see cref="IPainter" /> that can be used to draw graphical
        /// contents into this bitmap.
        /// </summary>
        /// <remarks>
        /// The returned <see cref="IPainter" /> acquires an exclusive lock to
        /// the contents of the bitmap. You must call <see cref="IPainter.End" />
        /// after you are done with using it.
        /// </remarks>
        /// <returns>A newly created <see cref="IPainter" /></returns>
        IPainter CreatePainter();
    }
}