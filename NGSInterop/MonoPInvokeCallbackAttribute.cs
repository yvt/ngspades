using System;
namespace Ngs.Interop
{
	// 
	// http://answers.unity3d.com/questions/191234/unity-ios-function-pointers.html
	public class MonoPInvokeCallbackAttribute
	{
		private Type type;

		public MonoPInvokeCallbackAttribute(Type t)
		{
			type = t;
		}
	}
}
