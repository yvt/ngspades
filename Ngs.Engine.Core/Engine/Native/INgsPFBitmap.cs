//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Canvas;

namespace Ngs.Engine.Native {
    /// <summary>
    /// Represents a mutable bitmap image.
    /// </summary>
    [Guid("d9cd03d0-3d98-481e-8668-e80e35b1f0b8")]
    public interface INgsPFBitmap : IUnknown {
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
        /// Create a <see cref="INgsPFPainter" /> that can be used to draw graphical
        /// contents into this bitmap.
        /// </summary>
        /// <remarks>
        /// The returned <see cref="INgsPFPainter" /> acquires an exclusive lock to
        /// the contents of the bitmap. You must call <see cref="INgsPFPainter.Finish" />
        /// after you are done with using it.
        /// </remarks>
        /// <returns>A newly created <see cref="INgsPFPainter" /></returns>
        INgsPFPainter CreatePainter();

        /// <summary>
        /// Creates a clone of the bitmap.
        /// </summary>
        /// <returns>A new bitmap containing the identical contents.</returns>
        INgsPFBitmap Clone();

        /// <summary>
        /// Converts this bitmap to an immutable image, invaludating this bitmap object.
        /// </summary>
        /// <returns>A new image object containing the identical contents.</returns>
        IUnknown IntoImage();

        /// <summary>
        /// Retrieves a pointer of the raw contents.
        /// </summary>
        IntPtr Contents { get; }

        /// <summary>
        /// Acquires a lock on the bitmap.
        /// </summary>
        void Lock();

        /// <summary>
        /// Releases a lock on the bitmap.
        /// </summary>
        void Unlock();
    }
}