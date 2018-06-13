//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.UI {
    /// <summary>
    /// Provides event data for the mounted/unmounted events.
    /// </summary>
    public class MountEventArgs {
        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        /// <param name="window">The value of <see cref="MountEventArgs.Window" />.</param>
        public MountEventArgs(Window window) {
            this.Window = window;
        }

        /// <summary>
        /// Retrieves the window which a view was mounted to/unmounted from.
        /// </summary>
        /// <returns>The window which a view was mounted to/unmounted from.</returns>
        public Window Window { get; set; }
    }
}
