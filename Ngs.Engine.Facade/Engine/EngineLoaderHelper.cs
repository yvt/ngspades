//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;

namespace Ngs.Engine {
    delegate int NgsLoaderGetProcessorInfo(out IntPtr retval);

    class EngineLoaderHelper {
        static DynamicLibrary dynamicLibrary;
        static Exception loadError;

        static EngineLoaderHelper() {
            try {
                if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows)) {
                    dynamicLibrary = DynamicLibrary.Load("libngsloader.dll");
                } else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX)) {
                    dynamicLibrary = DynamicLibrary.Load("libngsloader.dylib");
                } else {
                    dynamicLibrary = DynamicLibrary.Load("libngsloader.so");
                }
            } catch (Exception ex) {
                loadError = ex;
            }
        }

        static void EnsureLoaded() {
            if (loadError != null) {
                throw new EngineLoaderException("The engine loader helper library failed to load.",
                    loadError);
            }
        }

        public static IProcessorInfo ProcessorInfo {
            get {
                EnsureLoaded();

                var entryPtr = dynamicLibrary.GetSymbol("ngsloader_get_processor_info");
                if (entryPtr == IntPtr.Zero) {
                    throw new Exception("Could not find the engine loader helper entry point.");
                }

                var entryDelegate = Marshal.GetDelegateForFunctionPointer<NgsLoaderGetProcessorInfo>(entryPtr);
                Marshal.ThrowExceptionForHR(entryDelegate(out var processorInfoPtr));

                return NgscomMarshal.GetRcwForInterfacePtr<IProcessorInfo>(processorInfoPtr, false);
            }
        }
    }
}
