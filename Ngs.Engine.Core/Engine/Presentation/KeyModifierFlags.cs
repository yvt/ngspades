//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.Presentation {
    /// <summary>
    /// Indicates the state of modifier keys.
    /// </summary>
    [Flags]
    public enum KeyModifierFlags {
        /// <summary>
        /// The shift key.
        /// </summary>
        Shift = 1 << 0,

        /// <summary>
        /// The control key.
        /// </summary>
        Control = 1 << 1,

        /// <summary>
        /// The alt key.
        /// </summary>
        Alt = 1 << 2,

        /// <summary>
        /// The meta key (e.g., the Windows logo key in Windows, Command in macOS).
        /// </summary>
        Meta = 1 << 3,
    }
}