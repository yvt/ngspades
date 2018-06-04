//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Ngs.Utils {
    abstract class DynamicLibrary : IDisposable {
        public static DynamicLibrary Load(string path) {
            // TODO: Support Linux
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows)) {
                return new WindowsDynamicLibrary(path);
            } else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX)) {
                return new MacOSDynamicLibrary(path);
            } else {
                throw new Exception("Could not determine the current operating system type.");
            }
        }

        public virtual void Dispose() { }

        public abstract IntPtr GetSymbol(string name);
    }

    sealed class WindowsDynamicLibrary : DynamicLibrary {
        IntPtr hModule;

        [DllImport("kernel32.dll", SetLastError = true)]
        static extern IntPtr LoadLibrary(string path);
        [DllImport("kernel32.dll")]
        static extern IntPtr GetProcAddress(IntPtr hModule, string name);
        [DllImport("kernel32.dll")]
        static extern bool FreeLibrary(IntPtr hModule);

        public WindowsDynamicLibrary(string path) {
            hModule = LoadLibrary(path);
            if (hModule == IntPtr.Zero) {
                Marshal.ThrowExceptionForHR(Marshal.GetHRForLastWin32Error());
            }
        }

        public override IntPtr GetSymbol(string name) {
            return GetProcAddress(hModule, name);
        }

        #region IDisposable Support
        private bool disposedValue = false; // To detect redundant calls

        void Dispose(bool disposing) {
            if (!disposedValue) {
                FreeLibrary(hModule);
                disposedValue = true;
            }
        }

        ~WindowsDynamicLibrary() {
            // Do not change this code. Put cleanup code in Dispose(bool disposing) above.
            Dispose(false);
        }

        // This code added to correctly implement the disposable pattern.
        public override void Dispose() {
            // Do not change this code. Put cleanup code in Dispose(bool disposing) above.
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        #endregion
    }

    sealed class MacOSDynamicLibrary : DynamicLibrary {
        IntPtr handle;

        [DllImport("dl")]
        static extern IntPtr dlopen([In] byte[] path, int mode);
        [DllImport("dl")]
        static extern IntPtr dlsym(IntPtr handle, [In] byte[] symbol);
        [DllImport("dl")]
        static extern int dlclose(IntPtr handle);
        [DllImport("dl")]
        static extern IntPtr dlerror();

        static UTF8Encoding utf8 = new UTF8Encoding();

        static void ThrowDlError() {
            var errorMessagePtr = dlerror();
            if (errorMessagePtr == IntPtr.Zero) {
                throw new Exception("An unknown error has occured in the dynamic library loader.");
            } else {
                throw new Exception(Marshal.PtrToStringAnsi(errorMessagePtr));
            }
        }

        public MacOSDynamicLibrary(string path) {
            dlerror();
            handle = dlopen(utf8.GetBytes(path + "\0"), 0);
            if (handle == IntPtr.Zero) {
                ThrowDlError();
            }
        }

        public override IntPtr GetSymbol(string name) {
            return dlsym(handle, utf8.GetBytes(name + "\0"));
        }

        #region IDisposable Support
        private bool disposedValue = false; // To detect redundant calls

        void Dispose(bool disposing) {
            if (!disposedValue) {
                if (dlclose(handle) != 0) {
                    ThrowDlError();
                }
                disposedValue = true;
            }
        }

        ~MacOSDynamicLibrary() {
            // Do not change this code. Put cleanup code in Dispose(bool disposing) above.
            Dispose(false);
        }

        // This code added to correctly implement the disposable pattern.
        public override void Dispose() {
            // Do not change this code. Put cleanup code in Dispose(bool disposing) above.
            Dispose(true);
            GC.SuppressFinalize(this);
        }

        #endregion
    }


}