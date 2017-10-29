//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;

namespace Ngs.Engine.Presentation
{
    [Flags]
    public enum KeyModifierFlags
    {
        Shift = 1 << 0,
        Control = 1 << 1,
        Alt = 1 << 2,
        Meta = 1 << 3,
    }
}
