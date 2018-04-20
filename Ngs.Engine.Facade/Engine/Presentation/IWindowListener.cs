//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Presentation {
    /// <summary>
    /// Receives and handles window events.
    /// </summary>
    [Guid("bca93091-5031-4b44-ab90-fedd2a6b6692")]
    public interface IWindowListener : IUnknown {
        /// <summary>
        /// The window was resized.
        /// </summary>
        /// <param name="size">The new size of the window, measured in device independent pixels.</param>
        void Resized(Vector2 size);

        /// <summary>
        /// The window was moved.
        /// </summary>
        /// <param name="position">The new position of the window, measured in device independent pixels.</param>
        void Moved(Vector2 position);

        /// <summary>
        /// The user clicked the close button of the window.
        /// </summary>
        void Close();

        /// <summary>
        /// The window got or lost a focus.
        /// </summary>
        /// <param name="focused">
        /// Indicates whether the window got a focus (<c>true</c>) or lost a focus (<c>false</c>).
        /// </param>
        void Focused(bool focused);

        /// <summary>
        /// A mouse button was pressed or released.
        /// </summary>
        /// <param name="position">The position of the mouse cursor.</param>
        /// <param name="button">
        /// Identifies the mouse button (e.g. <c>0</c> and <c>1</c> for the left and right button, respectively).
        /// </param>
        /// <param name="pressed">
        /// Indicates whether the button was pressed (<c>true</c>) or released (<c>false</c>).
        /// </param>
        void MouseButton(MousePosition position, byte button, bool pressed);

        /// <summary>
        /// A mouse cursor moved on the window.
        /// </summary>
        /// <param name="position">The position of the mouse cursor.</param>
        void MouseMotion(MousePosition position);

        /// <summary>
        /// A mouse cursor left the client region of the window.
        /// </summary>
        void MouseLeave();

        /// <summary>
        /// A key was pressed or released.
        /// </summary>
        /// <param name="virtualKeyCode">
        /// The name identifying the key. This is currently derived from the enumerate item names of
        /// <c>winit</c>'s <c>VirtualKeyCode</c>.
        /// </param>
        /// <param name="pressed">
        /// Indicates whether the button was pressed (<c>true</c>) or released (<c>false</c>).
        /// </param>
        /// <param name="modifier">The state of modifier keys.</param>
        void KeyboardInput(string virtualKeyCode, bool pressed, KeyModifierFlags modifier);
    }
}