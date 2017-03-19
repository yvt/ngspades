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

			Console.WriteLine("Benchmarking emptymethod()");
			Benchmark(() => { });

			Console.WriteLine("Benchmarking IHoge.Hoge()");
			Benchmark(((IHoge)(new HogeClass())).Hoge);

			Console.WriteLine("Benchmarking obj.SimpleMethod()");
			Benchmark(obj.SimpleMethod);
		}
	}
}
