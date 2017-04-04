using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("c68f8abd-f88c-41e5-88da-956c3ca462e6")]
	public interface ITextCompositionEvent : IUnknown
	{
		string Text { get; }
		int SelectionStart { get; }
		int SelectionLength { get; }
	}

}
