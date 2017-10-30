//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.Presentation
{
    /// <summary>
    /// Indicates the state of modifier keys.
    /// </summary>
    [Flags]
    public enum KeyModifierFlags
    {
        /// <summary>
        /// The "Shift" key.
        /// </summary>
        Shift = 1 << 0,

        /// <summary>
        /// The "Control" key.
        /// </summary>
        Control = 1 << 1,

        /// <summary>
        /// The "Alt" key.
        /// </summary>
        Alt = 1 << 2,

        /// <summary>
        /// The "Meta" key (e.g., the Windows logo key in Windows, Command in macOS).
        /// </summary>
        Meta = 1 << 3,
    }
}
