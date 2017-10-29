using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Utils
{
    [StructLayout(LayoutKind.Sequential)]
    public struct Box3
    {
        private Vector3 min, max;

        public Box3(Vector3 min, Vector3 max)
        {
            this.min = min;
            this.max = max;
        }

        public Vector3 Min
        {
            get { return this.min; }
            set { this.min = value; }
        }

        public Vector3 Max
        {
            get { return this.max; }
            set { this.max = value; }
        }
    }
}
