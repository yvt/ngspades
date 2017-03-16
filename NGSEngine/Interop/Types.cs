using System.Runtime.InteropServices;

[assembly: PrimaryInteropAssembly(0, 0)]
[assembly: ImportedFromTypeLib("NGSCore")]

namespace Ngs.Engine
{
    [StructLayout(LayoutKind.Sequential)]
    public struct TerrainVoxelInfo
    {
        private uint color;
        private ushort kind;
        private byte health;

        TerrainVoxelInfo(uint color, ushort kind, byte health)
        {
            this.color = color;
            this.kind = kind;
            this.health = health;
        }

        public uint Color
        {
            get { return this.color; }
            set { this.color = value; }
        }

        public ushort KindID
        {
            get { return this.kind; }
            set { this.kind = value; }
        }

        public byte Health
        {
            get { return this.health; }
            set { this.health = value; }
        }
    }
}

namespace Ngs.Utils
{
    [StructLayout(LayoutKind.Sequential)]
    public struct IntVector3
    {
        private int x, y, z;

        IntVector3(int x, int y, int z)
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
}
