using System;
using System.Runtime.InteropServices;

namespace Ngs.Shell
{
	static class NativeMethods
	{
		[DllImport("NGSEngine", PreserveSig = false)]
		public static extern void NgsCreateTestInstance([MarshalAs(UnmanagedType.Interface)] out Ngs.Engine.ITestInterface outInstance);
	}

	sealed unsafe class TestInterfaceRCW
	{
		private IntPtr self;

        [System.Security.SuppressUnmanagedCodeSecurity]
        private delegate int SimpleMethodDelegate(IntPtr selfptr);
		private SimpleMethodDelegate simpleMethodImpl;
        
		private delegate int HelloDelegate(IntPtr selfptr, [MarshalAs(UnmanagedType.BStr)] string prm,
		                                  [MarshalAs(UnmanagedType.BStr)] out string retval);
		private HelloDelegate helloImpl;

		public TestInterfaceRCW(Ngs.Engine.ITestInterface obj)
		{
			IntPtr unk = Marshal.GetIUnknownForObject(obj);
			IntPtr iface;
			Guid guid = new Guid("35edff15-0b38-47d8-9b7c-e00fa2acdf9d");
			Marshal.QueryInterface(unk, ref guid, out iface);

			IntPtr** ifacePtr = (IntPtr**) iface.ToPointer();
			IntPtr* vtable = ifacePtr[0];

			IntPtr hello = vtable[5];
			helloImpl = Marshal.GetDelegateForFunctionPointer<HelloDelegate>(hello);

			IntPtr simpleMethod = vtable[6];
			simpleMethodImpl = Marshal.GetDelegateForFunctionPointer<SimpleMethodDelegate>(simpleMethod);

			this.self = iface;
		}

		public string Hello(string s)
		{
			string retval;
			if (helloImpl(self, s, out retval) < 0)
			{
				throw new COMException();
			}
			return retval;
		}

		public void SimpleMethod()
		{
			if (simpleMethodImpl(self) < 0)
			{
				throw new COMException();
			}
		}
	}

	class MainClass
	{
		interface IHoge
		{
			void Hoge();
		}
		class HogeClass : IHoge
		{
			public void Hoge()
			{
			}
		}

		private static void Benchmark(Action fn)
		{
			var sw = new System.Diagnostics.Stopwatch();
			long totalIter = 0;
			fn();
			sw.Start();
			while (sw.ElapsedMilliseconds < 1000)
			{
				for (int i = 0; i < 100000; ++i)
				{
					fn();
				}
				totalIter += 100000;
			}
			Console.WriteLine($"Result: {totalIter / sw.Elapsed.TotalSeconds} call/sec");
		}

		public static void Main(string[] args)
		{
			Ngs.Engine.ITestInterface obj;
			NativeMethods.NgsCreateTestInstance(out obj);

			Console.WriteLine("Entering obj.Hello()");
			string ret = obj.Hello("Message from managed code");
			Console.WriteLine("Leaving obj.Hello()");

			Console.WriteLine($"Got: \"{ret}\" (length = {ret.Length})");

			obj.HogeAttr = "Test value";
			ret = obj.HogeAttr;
			Console.WriteLine($"HogeAttr: \"{ret}\" (length = {ret.Length})");

			Console.WriteLine("Benchmarking IHoge.Hoge()");
			Benchmark(((IHoge)(new HogeClass())).Hoge);

			Console.WriteLine("Benchmarking obj.SimpleMethod()");
			Benchmark(obj.SimpleMethod);

			Console.WriteLine("-- Testing custom RCW");
			var rcw = new TestInterfaceRCW(obj);

			Console.WriteLine("Entering obj.Hello()");
			ret = rcw.Hello("Message from managed code (with custom RCW)");
			Console.WriteLine("Leaving obj.Hello()");

			Console.WriteLine($"Got: \"{ret}\" (length = {ret.Length})");

			Console.WriteLine("Benchmarking obj.SimpleMethod()");
			Benchmark(rcw.SimpleMethod);
		}
	}
}
