//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.UI.Input {
    /// <summary>
    /// Represents a (possibly emulated) mouse button.
    /// </summary>
    public abstract class MouseButton {
        /// <summary>
        /// Retrieves a <see cref="MouseButtonType" /> value that specifies the standard behavior of
        /// this mouse button.
        /// </summary>
        /// <returns>The <see cref="MouseButtonType" /> value, or <c>null</c> if this button does
        /// not have a standardized meaning.</returns>
        public virtual MouseButtonType? Type => null;

        /// <summary>
        /// Retrieves the displayed name (e.g. <c>"Left"</c>) of this mouse button.
        /// </summary>
        /// <returns>The displayed name of this mouse button.</returns>
        public abstract string Name { get; }
    }
}