using System.Runtime.InteropServices;

[assembly: PrimaryInteropAssembly(0, 0)]
[assembly: ImportedFromTypeLib("NGSCore")]

namespace Ngs.Engine
{
    public enum FullScreenMode : int
    {
        Windowed = 0,
        FullScreenWindow,
        FullScreen
    }

    public enum WheelDeltaMode : int
    {
        Pixel = 0,
        Line,
        Page
    }

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
