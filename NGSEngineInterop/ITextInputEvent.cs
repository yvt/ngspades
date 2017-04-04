using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("22a11e7e-7dfe-42d6-9c4c-9f23d4d7fb75")]
	public interface ITextInputEvent : IUnknown
	{
		string Text { get; }
	}

}
