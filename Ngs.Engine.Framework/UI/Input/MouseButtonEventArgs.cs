//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.UI.Input {
    /// <summary>
    /// Provides event data for the mouse button events.
    /// </summary>
    public class MouseButtonEventArgs : MouseEventArgs {
        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        /// <param name="position">The value of <see cref="MouseEventArgs.Position" />.</param>
        /// <param name="button">The value of <see cref="Button" />.</param>
        public MouseButtonEventArgs(Point position, MouseButton button) : base(position) {
            this.Button = button;
        }

        /// <summary>
        /// Retrieves the mouse button whose state has changed.
        /// </summary>
        /// <returns>The mouse button.</returns>
        public MouseButton Button { get; }
    }
}