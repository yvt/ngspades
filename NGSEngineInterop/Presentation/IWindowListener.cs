using System;
using System.Numerics;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Presentation
{
	[Guid("bca93091-5031-4b44-ab90-fedd2a6b6692")]
	public interface IWindowListener : IUnknown
	{
		void Resized(Vector2 size);
        void Moved(Vector2 position);
        void Close();
        void Focused(bool focused);
        void MouseButton(MousePosition position, byte button, bool pressed);
        void MouseMotion(MousePosition position);
        void MouseLeave();
        void KeyboardInput(string virtualKeyCode, bool pressed, KeyModifierFlags modifier);
	}
}
