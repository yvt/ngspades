using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Presentation
{
	[Guid("a1bada5d-c290-49b2-9807-6adb46fe545b")]
	public interface IPresentationFramework : IUnknown
	{
        INodeGroup CreateNodeGroup();
        IWindow CreateWindow();
        ILayer CreateLayer();
	}
}
