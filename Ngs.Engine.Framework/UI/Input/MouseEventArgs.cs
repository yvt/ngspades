//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.UI.Input {
    /// <summary>
    /// Provides event data for the mouse events.
    /// </summary>
    public class MouseEventArgs {
        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        /// <param name="position">The value of <see cref="Position" />.</param>
        public MouseEventArgs(Point position) {
            this.Position = position;
        }

        /// <summary>
        /// Retrieves the position where the event has occured.
        /// </summary>
        /// <returns>The position where the event has occured.</returns>
        public Point Position { get; }
    }
}