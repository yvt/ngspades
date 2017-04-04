using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("570e1466-cd26-4363-acb8-00eb73269497")]
	public interface IViewportListener : IUnknown
	{
		void KeyDown([MarshalAs(UnmanagedType.Interface)] IKeyboardEvent keyboardEvent);
		void KeyUp([MarshalAs(UnmanagedType.Interface)] IKeyboardEvent keyboardEvent);
		void TextCompose([MarshalAs(UnmanagedType.Interface)] ITextCompositionEvent textCompositionEvent);
		void TextInput([MarshalAs(UnmanagedType.Interface)] ITextInputEvent textInputEvent);
		void MouseDown([MarshalAs(UnmanagedType.Interface)] IMouseEvent mouseEvent);
		void MouseMove([MarshalAs(UnmanagedType.Interface)] IMouseEvent mouseEvent);
		void MouseUp([MarshalAs(UnmanagedType.Interface)] IMouseEvent mouseEvent);
		void MouseWheel([MarshalAs(UnmanagedType.Interface)] IMouseEvent mouseEvent);
	}

}
