using System;
using System.Runtime.InteropServices;

namespace Ngs.Shell
{
	static class NativeMethods
	{
		[DllImport("NGSEngine", PreserveSig = false)]
		public static extern void NgsCreateTestInstance([MarshalAs(UnmanagedType.Interface)] out Ngs.Engine.ITestInterface outInstance);
	}

	class MainClass
	{
		public static void Main(string[] args)
		{
			Ngs.Engine.ITestInterface obj;
			NativeMethods.NgsCreateTestInstance(out obj);
			Console.WriteLine("Entering obj.Hello()");
			obj.Hello("Message from managed code");
			Console.WriteLine("Leaving obj.Hello()");
		}
	}
}
