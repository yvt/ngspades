using System.Runtime.InteropServices;

namespace Ngs.Utils
{
    [StructLayout(LayoutKind.Sequential)]
    public struct Vector2
    {
        private float x, y;

        public Vector2(float x, float y)
        {
            this.x = x;
            this.y = y;
        }

        public float X
        {
            get { return this.x; }
            set { this.x = value; }
        }

        public float Y
        {
            get { return this.y; }
            set { this.y = value; }
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Vector3
    {
        private float x, y, z;

        public Vector3(float x, float y, float z)
        {
            this.x = x;
            this.y = y;
            this.z = z;
        }

        public float X
        {
            get { return this.x; }
            set { this.x = value; }
        }

        public float Y
        {
            get { return this.y; }
            set { this.y = value; }
        }

        public float Z
        {
            get { return this.z; }
            set { this.z = value; }
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Vector4
    {
        private float x, y, z, w;

        public Vector4(float x, float y, float z, float w)
        {
            this.x = x;
            this.y = y;
            this.z = z;
            this.w = w;
        }

        public float X
        {
            get { return this.x; }
            set { this.x = value; }
        }

        public float Y
        {
            get { return this.y; }
            set { this.y = value; }
        }

        public float Z
        {
            get { return this.z; }
            set { this.z = value; }
        }

        public float W
        {
            get { return this.w; }
            set { this.w = value; }
        }
    }

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

    [StructLayout(LayoutKind.Sequential)]
    public struct IntVector3
    {
        private int x, y, z;

        public IntVector3(int x, int y, int z)
        {
            this.x = x;
            this.y = y;
            this.z = z;
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
    }

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
