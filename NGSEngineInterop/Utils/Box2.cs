//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Utils
{
    [StructLayout(LayoutKind.Sequential)]
    public struct Box2
    {
        private Vector2 min, max;

        public Box2(Vector2 min, Vector2 max)
        {
            this.min = min;
            this.max = max;
        }

        public Vector2 Min
        {
            get { return this.min; }
            set { this.min = value; }
        }

        public Vector2 Max
        {
            get { return this.max; }
            set { this.max = value; }
        }
    }
}
