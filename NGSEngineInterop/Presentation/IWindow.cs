using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;

namespace Ngs.Engine.Presentation
{
	[Guid("1fd3658b-e4ac-49bb-9609-a0e578022cbc")]
	public interface IWindow : IUnknown
	{
		WindowFlags Flags { set; }
        Vector2 Size { set; }
        IUnknown Child { set; }
        string Title { set; }
        IWindowListener Listener { set; }
	}
}
