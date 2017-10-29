using System;

namespace Ngs.Engine.Presentation
{
    [Flags]
    public enum WindowFlags
    {
        Resizable = 1 << 0,
        Borderless = 1 << 1,
        Transparent = 1 << 2,
        DenyUserClose = 1 << 3,
    }
}
