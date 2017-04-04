using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("c77ded14-6119-45a4-b650-dea3f641862a")]
	public interface IViewport : IUnknown
	{
		void AddListener(IViewportListener listener);
		void RemoveListener(IViewportListener listener);

		int VideoWidth { get; }
		int VideoHeight { get; }
		Ngs.Engine.FullScreenMode FullScreenMode { get; }
		float DevicePixelRatio { get; }
		void SetVideoMode(int videoWidth, int videoHeight, Ngs.Engine.FullScreenMode fullScreenMode, bool useNativePixelRatio);

		bool EnableTextInput { get; set; }
		Ngs.Utils.Box2D TextInputRectangle { get; set; }
	}

}
