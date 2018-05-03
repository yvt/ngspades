//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.IO;
using System.Runtime.InteropServices;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Native;

namespace Ngs.Engine {
    delegate int NgsLoaderGetProcessorInfo(out IntPtr retval);

    static class EngineLoaderHelper {
        static DynamicLibrary dynamicLibrary;
        static Exception loadError;
        public static string EnginePath { get; }

        static EngineLoaderHelper() {
            EnginePath = Path.GetDirectoryName(
                    System.Reflection.Assembly.GetExecutingAssembly().Location);

            string envValue = Environment.GetEnvironmentVariable("NGS_ENGINE_PATH");
            if (!string.IsNullOrWhiteSpace(envValue)) {
                EnginePath = envValue;
            }

            try {
                string helperPath;
                if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows)) {
                    helperPath = Path.Combine(EnginePath, "ngsloader.dll");
                } else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX)) {
                    helperPath = Path.Combine(EnginePath, "libngsloader.dylib");
                } else {
                    helperPath = Path.Combine(EnginePath, "libngsloader.so");
                }

                dynamicLibrary = DynamicLibrary.Load(helperPath);
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

        public static INgsProcessorInfo ProcessorInfo {
            get {
                EnsureLoaded();

                var entryPtr = dynamicLibrary.GetSymbol("ngsloader_get_processor_info");
                if (entryPtr == IntPtr.Zero) {
                    throw new Exception("Could not find the engine loader helper entry point.");
                }

                var entryDelegate = Marshal.GetDelegateForFunctionPointer<NgsLoaderGetProcessorInfo>(entryPtr);
                Marshal.ThrowExceptionForHR(entryDelegate(out var processorInfoPtr));

                return NgscomMarshal.GetRcwForInterfacePtr<INgsProcessorInfo>(processorInfoPtr, false);
            }
        }
    }
}
