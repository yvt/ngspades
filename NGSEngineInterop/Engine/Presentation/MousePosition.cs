//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Engine.Presentation {
    /// <summary>
    /// Position data associated with a mouse input event.
    /// </summary>
    [StructLayout (LayoutKind.Sequential)]
    public struct MousePosition {
        private Vector2 client;
        private Vector2 global;

        /// <summary>
        /// Sets or retrives the mouse cursor position (measured in device independent pixels)
        /// in the client coordinate space.
        /// </summary>
        public Vector2 Client {
            get { return this.client; }
            set { this.client = value; }
        }

        /// <summary>
        /// Sets or retrives the mouse cursor position (measured in device independent pixels)
        /// in the global coordinate space.
        /// </summary>
        /// <remarks>
        /// The global coordinate space is a coordinate space that is invariant regard to
        /// the window's position.
        /// </remarks>
        public Vector2 Global {
            get { return this.global; }
            set { this.global = value; }
        }
    }
}