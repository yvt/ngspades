using System;
namespace Ngs.Interop
{
	public static class NgscomMarshal
	{
		[System.Security.SecurityCritical]
		public static T GetRCWForInterfacePtr<T>(IntPtr ptr, bool addRef) where T : class, IUnknown
		{
			return InterfaceRuntimeInfo<T>.CreateRCW(ptr, addRef);
		}

		public static T QueryInterfaceOrNull<T>(IUnknown obj) where T : class, IUnknown
		{
			var nativeObj = obj as INativeObject<T>;
			if (nativeObj != null)
			{
				try
				{
					var guid = InterfaceRuntimeInfo<T>.ComGuid;
					var ptr = nativeObj.QueryNativeInterface(ref guid);
					return InterfaceRuntimeInfo<T>.CreateRCW(ptr, true);
				}
				catch (System.Runtime.InteropServices.COMException ex) // TODO: use C# 7.0 filter clause
				{
					const int E_NOINTERFACE = unchecked((int)0x80004002);
					if (ex.HResult == E_NOINTERFACE)
					{
						return null;
					}
					else
					{
						throw;
					}
				}
			}
			return obj as T;
		}

		public static T QueryInterface<T>(IUnknown obj) where T : class, IUnknown
		{
			var nativeObj = obj as INativeObject<T>;
			if (nativeObj != null)
			{
				var guid = InterfaceRuntimeInfo<T>.ComGuid;
				var ptr = nativeObj.QueryNativeInterface(ref guid);
				return InterfaceRuntimeInfo<T>.CreateRCW(ptr, true);
			}
			return (T) obj;
		}
	}
}
