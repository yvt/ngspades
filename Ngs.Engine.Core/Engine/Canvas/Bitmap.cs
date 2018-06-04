//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using Ngs.Engine;
using Ngs.Interop;
using Ngs.Engine.Native;

namespace Ngs.Engine.Canvas {
    /// <summary>
    /// Represents a method that accesses the raw contents of a <see cref="Bitmap" />.
    /// </summary>
    /// <param name="bitmap">The bitmap to access the raw contents of.</param>
    /// <param name="rawContents">A reference to the raw contents.</param>
    /// <returns>A custom return value.</returns>
    public delegate T BitmapContentsAccessor<T>(Bitmap bitmap, Span<byte> rawContents);

    /// <summary>
    /// Represents a mutable bitmap image.
    /// </summary>
    /// <remarks>
    /// <para>This class is a wrapper of <see cref="INgsPFBitmap" />.</para>
    /// <para>All members of this class are thread-safe, with the exception of <see cref="ToImage" />
    /// and <see cref="IntoImage" /> that mutate the internal state of a native bitmap object.</para>
    /// </remarks>
    public sealed class Bitmap {
        private INgsPFBitmap nativeBitmap;

        /// <summary>
        /// Constructs a new instance of <see cref="Bitmap" />.
        /// </summary>
        /// <param name="size">The size of a newly created bitmap.</param>
        /// <param name="format">The pixel format of a newly created bitmap.</param>
        [SecuritySafeCritical]
        public Bitmap(IntVector2 size, PixelFormat format) {
            nativeBitmap = EngineInstance.NativeEngine.CreateBitmap(size, format);
        }

        /// <summary>
        /// Constructs a new instance of <see cref="Bitmap" />.
        /// </summary>
        /// <param name="width">The width of a newly created bitmap.</param>
        /// <param name="height">The height of a newly created bitmap.</param>
        /// <param name="format">The pixel format of a newly created bitmap.</param>
        public Bitmap(int width, int height, PixelFormat format) :
            this(new IntVector2(width, height), format) {
        }

        internal Bitmap(INgsPFBitmap nativeBitmap) {
            this.nativeBitmap = nativeBitmap;
        }

        internal INgsPFBitmap NativeBitmap {
            get => nativeBitmap ??
                throw new InvalidOperationException("The bitmap has already been destructively " +
                    "converted into an (immutable) image object.");
        }

        #region Forwarded to this.nativeBitmap

        /// <summary>
        /// Retrieves the size of the bitmap.
        /// </summary>
        /// <returns>The size of the bitmap, measured in pixels.</returns>
        public IntVector2 Size { get => NativeBitmap.Size; }

        /// <summary>
        /// Retrieves the width of the bitmap.
        /// </summary>
        /// <returns>The width of the bitmap, measured in pixels.</returns>
        public int Width { get => Size.X; }

        /// <summary>
        /// Retrieves the height of the bitmap.
        /// </summary>
        /// <returns>The height of the bitmap, measured in pixels.</returns>
        public int Height { get => Size.Y; }

        /// <summary>
        /// Retrieves the pixel format of the bitmap.
        /// </summary>
        /// <returns>The pixel format of the bitmap.</returns>
        public PixelFormat Format { get => NativeBitmap.Format; }

        /// <summary>
        /// Create a <see cref="INgsPFPainter" /> that can be used to draw graphical
        /// contents into this bitmap.
        /// </summary>
        /// <remarks>
        /// The returned <see cref="INgsPFPainter" /> holds an exclusive lock to
        /// the contents of the bitmap. You must call <see cref="Painter.Dispose" />
        /// after you are done with using it.
        /// </remarks>
        /// <returns>A newly created <see cref="INgsPFPainter" /></returns>
        public Painter CreatePainter() => new Painter(NativeBitmap.CreatePainter());

        /// <summary>
        /// Creates a clone of the bitmap.
        /// </summary>
        /// <returns>A new bitmap containing the identical contents.</returns>
        public Bitmap Clone() => new Bitmap(NativeBitmap.Clone());

        /// <summary>
        /// Converts this bitmap to an immutable image, invalidating this bitmap object.
        /// </summary>
        /// <remarks>
        /// This method transitions the image data into the immutable state. The original bitmap
        /// object will be no longer usable.
        /// </remarks>
        /// <returns>A new (immutable) image object containing the identical contents.</returns>
        public Image IntoImage() {
            var image = new Image(NativeBitmap.IntoImage());
            nativeBitmap = null;
            return image;
        }

        #endregion

        /// <summary>
        /// Converts this bitmap to an immutable image.
        /// </summary>
        /// <remarks>
        /// In contrast to <see cref="IntoImage" />, this method preserves the original
        /// bitmap object. This involves cloning the image data, so use <see cref="IntoImage" />
        /// whenever possible.
        /// </remarks>
        /// <returns>A new i(immutable) mage object containing the identical contents.</returns>
        public Image ToImage() => new Image(nativeBitmap.Clone().IntoImage());

        /// <summary>
        /// Calls a supplied function with a reference to its raw contents.
        /// </summary>
        /// <remarks>
        /// <para>This method acquires an exclusive lock on the bitmap to access its raw
        /// contents. The lock is automatically released when the supplied function returns.</para>
        /// <para>The bitmap contents are located within an unmanaged memory region, and a memory
        /// error would result if its lifetime and ownership is not properly managed.
        /// This method ensures that the caller-supplied method <see paramref="callback" /> can
        /// access the contents in a memory-safe manner.</para>
        /// </remarks>
        /// <param name="callback">The method to be called with a reference to the raw contents.
        /// as its argument.</param>
        /// <returns>A custom value returned by <see paramref="callback" />.</returns>
        [SecuritySafeCritical]
        public unsafe T UsingContents<T>(BitmapContentsAccessor<T> callback) {
            var nativeBitmap = NativeBitmap;
            nativeBitmap.Lock();
            try {
                checked {
                    int numBytesPerPixel;
                    switch (nativeBitmap.Format) {
                        case PixelFormat.SrgbRgba8:
                        case PixelFormat.SrgbRgba8Premul:
                            numBytesPerPixel = 4;
                            break;
                        default:
                            throw new InvalidOperationException();
                    }

                    var size = nativeBitmap.Size;
                    int numBytes = size.X * size.Y * numBytesPerPixel;

                    var ptr = nativeBitmap.Contents;
                    var span = new Span<byte>((void*)ptr, numBytes);

                    return callback(this, span);
                }
            } finally {
                nativeBitmap.Unlock();
            }
        }
    }
}