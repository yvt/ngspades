using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Engine.Presentation
{
    [StructLayout(LayoutKind.Sequential)]
    public struct MousePosition 
    {
        private Vector2 client;
        private Vector2 global;

        public Vector2 Client
        {
            get { return this.client; }
            set { this.client = value; }
        }

        public Vector2 Global
        {
            get { return this.global; }
            set { this.global = value; }
        }
    }
}
