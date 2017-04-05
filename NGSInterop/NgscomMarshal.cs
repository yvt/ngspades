using System;
using System.Runtime.InteropServices;
namespace Ngs.Interop
{
	public static partial class NgscomMarshal
	{
		[System.Security.SecurityCritical]
		public static T GetRcwForInterfacePtr<T>(IntPtr ptr, bool addRef) where T : class, IUnknown
		{
			if (ptr == IntPtr.Zero) {
				return null;
			}
			return InterfaceRuntimeInfo<T>.CreateRcw(ptr, addRef);
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
					return InterfaceRuntimeInfo<T>.CreateRcw(ptr, true);
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
				return InterfaceRuntimeInfo<T>.CreateRcw(ptr, true);
			}
			return (T) obj;
		}
	}
}
