//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;

namespace Ngs.Engine.Canvas {
    /// <summary>
    /// An abstract interface used to issue draw operations.
    /// </summary>
    [Guid("009da12d-75f2-4714-9dfa-9ce0beeea97a")]
    public interface IPainter : IUnknown {
        /// <summary>
        /// Informs this painter that you have finished drawing using it.
        /// </summary>
        void Finish();

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
        void Lock();

        /// <summary>
        /// Releases an exclusive lock.
        /// </summary>
        void Unlock();

        /// <summary>
        /// Saves the current drawing state to the stack.
        /// </summary>
        void Save();

        /// <summary>
        /// Restores a drawing state from the stack. Fails if the stack is empty.
        /// </summary>
        void Restore();

        /// <summary>
        /// Translates the current transformation matrix by the specified amount.
        /// </summary>
        /// <param name="offset">The translation amount.</param>
        void Translate(Vector2 offset);

        /// <summary>
        /// Scale the current transformation matrix by the specified factor.
        /// </summary>
        /// <param name="x">The scaling factor for the X direction.</param>
        /// <param name="y">The scaling factor for the Y direction.</param>
        void NonUniformScale(float x, float y);

        /// <summary>
        /// Changes the current fill style to solid color, and use a given color
        /// for filling operations.
        /// </summary>
        /// <param name="color">The color used for filling operations.</param>
        void SetFillColor(Rgba color);

        // TODO: Text rendering
    }
}