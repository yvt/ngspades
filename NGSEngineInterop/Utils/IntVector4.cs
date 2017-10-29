using System.Runtime.InteropServices;
namespace Ngs.Utils
{
    [StructLayout(LayoutKind.Sequential)]
    public struct IntVector4
    {
        private int x, y, z, w;

        public IntVector4(int x, int y, int z, int w)
        {
            this.x = x;
            this.y = y;
            this.z = z;
            this.w = w;
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

        public int Z
        {
            get { return this.z; }
            set { this.z = value; }
        }

        public int W
        {
            get { return this.w; }
            set { this.w = value; }
        }
    }
}
