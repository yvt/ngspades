using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("74f49795-979e-42bf-a9c9-2a3cc83a916b")]
	public interface IKeyboardEvent : IUnknown
	{
		int ScanCode { get; }
	}

}
