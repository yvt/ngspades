//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Numerics;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;

namespace Ngs.Engine.Canvas {
    /// <summary>
    /// An abstract interface used to issue draw operations.
    /// </summary>
    /// <remarks>
    /// <para>This struct is a wrapper of <see cref="IPainter" /> and provides additional
    /// functionalities including:</para>
    /// <list type="bullet">
    ///     <item><term>
    ///     Overloaded methods provided for convenience.
    ///     </term></item>
    ///     <item><term>
    ///     An <see cref="IDisposable" /> implementation that calls <see cref="IPainter.Finish" />,
    ///     which allows application developers to use C♯'s <c>using</c> directive (or
    ///     equivalent constructs of other languages) to ensure the correct use of painter objects.
    ///     </term></item>
    /// </list>
    /// <para>All members of this class are thread-safe.</para>
    /// </remarks>
    public struct Painter : IDisposable {
        private IPainter nativePainter;

        internal Painter(IPainter nativePainter) {
            this.nativePainter = nativePainter;
        }

        /// <summary>
        /// Retrieves the underlying native painter object.
        /// </summary>
        /// <returns>The underlying native painter object.</returns>
        internal IPainter NativePainter {
            [SecurityCritical]
            get => nativePainter;
        }

        /// <summary>
        /// Declares that you have done using the painter and releases its associated resources and
        /// locks.
        /// </summary>
        /// <remarks>
        /// <para>Calling this releases unmanaged resources associated with the painter, along with
        /// the exclusive lock on the underlying object, such as <see cref="Bitmap" />. Most
        /// operations on the underlying object, for example, <see cref="Bitmap.IntoImage" />
        /// requires an exclusive lock on it. Therefore, it's imperative that you release the
        /// painter after you've done using it.</para>
        /// </remarks>
        public void Dispose() => nativePainter.Finish();

        /// <summary>
        /// A lock guard for <see cref="Painter" /> created by the <see cref="Lock" /> method.
        /// </summary>
        public struct LockGuard : IDisposable {
            Painter parent;
            internal LockGuard(Painter parent) {
                this.parent = parent;
            }

            /// <summary>
            /// Calls <see cref="Unlock" /> of the associated painter object.
            /// </summary>
            public void Dispose() {
                parent.Unlock();
            }
        }

        /// <summary>
        /// Acquires an exclusive lock for the current thread.
        /// </summary>
        /// <remarks>
        /// <para>A painter is protected from concurrent accesses using a single mutex. By default,
        /// a lock on this mutex is acquired every time you call a method. Since every lock
        /// operation incurs a moderate performance cost, you can alternatively choose to explicitly
        /// acquire a lock for an extended duration by using this method.</para>
        /// <para>The acquired lock will be linked to the current thread, and lasts until
        /// <see cref="Unlock" /> is called from the same thread.</para>
        /// </remarks>
        /// <returns>A <see cref="LockGuard" /> object that calls <see cref="Unlock" /> when
        /// disposed.</returns>
        public LockGuard Lock() {
            nativePainter.Lock();
            return new LockGuard(this);
        }

        /// <summary>
        /// Releases an exclusive lock.
        /// </summary>
        public void Unlock() => nativePainter.Unlock();

        /// <summary>
        /// Saves the current drawing state to the stack.
        /// </summary>
        public void Save() => nativePainter.Save();

        /// <summary>
        /// Restores a drawing state from the stack. Fails if the stack is empty.
        /// </summary>
        public void Restore() => nativePainter.Restore();

        /// <summary>
        /// Translates the current transformation matrix by the specified amount.
        /// </summary>
        /// <param name="offset">The translation amount.</param>
        public void Translate(Vector2 offset) => nativePainter.Translate(offset);

        /// <summary>
        /// Translates the current transformation matrix by the specified amount.
        /// </summary>
        /// <param name="x">The X component of the translation amount.</param>
        /// <param name="y">The Y component of the translation amount.</param>
        public void Translate(float x, float y) => nativePainter.Translate(new Vector2(x, y));

        /// <summary>
        /// Scales the current transformation matrix by the specified factors.
        /// </summary>
        /// <param name="x">The scaling factor for the X direction.</param>
        /// <param name="y">The scaling factor for the Y direction.</param>
        public void Scale(float x, float y) => nativePainter.NonUniformScale(x, y);

        /// <summary>
        /// Scales the current transformation matrix by the specified factor.
        /// </summary>
        /// <param name="x">The scaling factor.</param>
        public void Scale(float x) => nativePainter.NonUniformScale(x, x);

        /// <summary>
        /// Changes the current fill style to solid color, and use a given color
        /// for filling operations.
        /// </summary>
        /// <param name="color">The color used for filling operations.</param>
        public void SetFillColor(Rgba color) => nativePainter.SetFillColor(color);
    }
}