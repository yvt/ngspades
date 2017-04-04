using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("9a6d2519-a9f6-4fcb-9515-d82c5da55466")]
	public interface IMouseEvent : IUnknown
	{
		Ngs.Utils.Vector2 Location { get; }
		int Buttons { get; }
		int ChangedButtons { get; }
		Ngs.Utils.Vector2 WheelDelta { get; }
		Ngs.Engine.WheelDeltaMode WheelDeltaMode { get; }
	}

}
