//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.UI.Input {
    /// <summary>
    /// Provides event data for input events that occured on <see cref="View" />.
    /// </summary>
    /// <remarks>
    /// The input events implement a behavior called <em>bubbling</em> — events are automatically
    /// propagated to the superview unless the event handler explicitly disables this behavior by
    /// setting the <see cref="StopPropagation" /> property to <c>true</c>.
    /// </remarks>
    public class BubblingInputEventArgs {
        /// <summary>
        /// Sets or retrieves a flag indicating whether this event is propagated to the superview.
        /// </summary>
        /// <returns><c>true</c> if the event is not propagated to the superview; otherwise,
        /// <c>false</c>.</returns>
        public bool StopPropagation { get; set; }
    }
}