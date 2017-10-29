//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Runtime.InteropServices;

namespace Ngs.Utils
{
    [StructLayout(LayoutKind.Sequential)]
    public struct IntVector2
    {
        private int x, y;

        public IntVector2(int x, int y)
        {
            this.x = x;
            this.y = y;
        }

        public int X
        {
            get { return this.x; }
            set { this.x = value; }
        }

        public int Y
        {
            get { return this.y; }
            set { this.y = value; }
        }
    }
}
