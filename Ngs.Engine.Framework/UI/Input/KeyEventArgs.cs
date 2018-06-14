//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.ComponentModel;
using Ngs.Engine.Presentation;

namespace Ngs.Engine.UI.Input {
    /// <summary>
    /// Provides event data for the mouse events.
    /// </summary>
    public class KeyEventArgs : HandledEventArgs {
        /// <summary>
        /// Initializes a new instance of this class.
        /// </summary>
        /// <param name="virtualKeyCode">The value of <see cref="VirtualKeyCode" />.</param>
        /// <param name="modifiers">The value of <see cref="Modifiers" />.</param>
        public KeyEventArgs(VirtualKeyCode virtualKeyCode, KeyModifierFlags modifiers) {
            this.VirtualKeyCode = virtualKeyCode;
            this.Modifiers = modifiers;
        }

        /// <summary>
        /// Retrieves the symbolic name of the key that was pressed or released.
        /// </summary>
        /// <returns>The <see cref="VirtualKeyCode" /> value indicating the symbolic name of a key.
        /// </returns>
        public VirtualKeyCode VirtualKeyCode { get; }

        /// <summary>
        /// Retrieves flags indicating the key modifiers.
        /// </summary>
        /// <returns>The key modifier flags.</returns>
        public KeyModifierFlags Modifiers { get; }
    }
}
