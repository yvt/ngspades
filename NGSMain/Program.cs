using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Shell
{
    static class NativeMethods
    {
        [DllImport("ngsengine")]
        public static extern int create_test_instance(out IntPtr outInstance);
    }

    sealed unsafe class TestInterfaceRcw
    {
        private IntPtr self;

        private delegate int SimpleMethodDelegate(IntPtr selfptr);
        private SimpleMethodDelegate simpleMethodImpl;

        private delegate int HelloDelegate(IntPtr selfptr, [MarshalAs(UnmanagedType.BStr)] string prm,
                                          [MarshalAs(UnmanagedType.BStr)] out string retval);
        private HelloDelegate helloImpl;

        public TestInterfaceRcw(IntPtr iface)
        {
            /*
            IntPtr unk = Marshal.GetIUnknownForObject(obj);
            IntPtr iface;
            Guid guid = new Guid("35edff15-0b38-47d8-9b7c-e00fa2acdf9d");
            Marshal.QueryInterface(unk, ref guid, out iface); */

            IntPtr** ifacePtr = (IntPtr**) iface.ToPointer();
            IntPtr* vtable = ifacePtr[0];

            // FIXME: Doesn't work with .NET Core
            // System.Runtime.InteropServices.MarshalDirectiveException: Cannot marshal 'parameter #2': Unknown error
            // IntPtr hello = vtable[5];
            // helloImpl = Marshal.GetDelegateForFunctionPointer<HelloDelegate>(hello);

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
            int result = simpleMethodImpl(self);
            if (result < 0) {
                Marshal.ThrowExceptionForHR(result);
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

        private static void Benchmark(Action<int> fn)
        {
            var sw = new System.Diagnostics.Stopwatch();
            long totalIter = 0;
            fn(100);
            sw.Start();
            while (sw.ElapsedMilliseconds < 1000)
            {
                fn(100000);
                totalIter += 100000;
            }
            Console.WriteLine($"Result: {totalIter / sw.Elapsed.TotalSeconds} call/sec");
        }

        public static void Main(string[] args)
        {
            IntPtr obj;
            Console.WriteLine("Creating obj");
            NativeMethods.create_test_instance(out obj);

			var rcw = NgscomMarshal.GetRcwForInterfacePtr<Ngs.Engine.ITestInterface>(obj, false);
            Console.WriteLine("Entering obj.Hello()");
            string ret = rcw.Hello("Message from managed code");
            Console.WriteLine("Leaving obj.Hello()");

            Console.WriteLine($"Got: \"{ret}\" (length = {ret.Length})");

            rcw.HogeAttr = "Test value";
            ret = rcw.HogeAttr;
            Console.WriteLine($"HogeAttr: \"{ret}\" (length = {ret.Length})");

            Console.WriteLine("Benchmarking IHoge.Hoge()");
            var ihoge = (IHoge)(new HogeClass());
			Benchmark((count) => {
                var theObject = ihoge;
                for (int i = 0; i < count; ++i) {
                    theObject.Hoge();
                }
            });

			Console.WriteLine("Benchmarking obj.SimpleMethod()");
			Benchmark((count) => {
                var theObject = rcw;
                for (int i = 0; i < count; ++i) {
                    theObject.SimpleMethod();
                }
            });

            Console.WriteLine("-- Testing custom RCW");
            var rcw2 = new TestInterfaceRcw(obj);

            // FIXME: Doesn't work with .NET Core
            //Console.WriteLine("Entering obj.Hello()");
            //string ret = rcw.Hello("Message from managed code (with custom RCW)");
            //Console.WriteLine("Leaving obj.Hello()");

            //Console.WriteLine($"Got: \"{ret}\" (length = {ret.Length})");

            Console.WriteLine("Benchmarking obj.SimpleMethod()");
			Benchmark((count) => {
                var theObject = rcw2;
                for (int i = 0; i < count; ++i) {
                    theObject.SimpleMethod();
                }
            });
        }
    }
}
