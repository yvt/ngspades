//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;

namespace Ngs.UI {
    /// <summary>
    /// Represents a result of the measurement step of the layouting algorithm.
    /// </summary>
    public struct Measurement {
        public Vector2 MinimumSize { get; set; }
        public Vector2 MaximumSize { get; set; }
        public Vector2 PreferredSize { get; set; }

        public static readonly Measurement Default = new Measurement()
        {
            MaximumSize = new Vector2(float.PositiveInfinity, float.PositiveInfinity),
        };
    }
}
