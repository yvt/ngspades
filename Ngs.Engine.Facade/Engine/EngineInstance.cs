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
    delegate int NgsEngineCreate(out IntPtr retval);

    /// <summary>
    /// Loads the native code part of the game engine and provides an entry
    /// point to it.
    /// </summary>
    public static class EngineInstance {
        static DynamicLibrary dynamicLibrary;
        static IEngine nativeEngine;
        static Exception dynamicLibraryError;

        static EngineInstance() {
            try {
                // TODO: Dispatch the best binary depending on the processor's capability
                // TODO: Use the appropriate extension depending on the platform
                var library = DynamicLibrary.Load("libngsengine.dylib");

                var entryPtr = library.GetSymbol("ngsengine_create");
                if (entryPtr == IntPtr.Zero) {
                    throw new Exception("Could not find the engine entry point.");
                }

                var entryDelegate = Marshal.GetDelegateForFunctionPointer<NgsEngineCreate>(entryPtr);
                Marshal.ThrowExceptionForHR(entryDelegate(out var enginePtr));

                nativeEngine = NgscomMarshal.GetRcwForInterfacePtr<IEngine>(enginePtr, false);
                dynamicLibrary = library;
            } catch (Exception ex) {
                dynamicLibraryError = ex;
            }
        }

        /// <summary>
        /// Ensures the game engine is loaded. Throws an exception if there was
        /// an error while loading the game engine DLL.
        /// </summary>
        public static void EnsureLoaded() {
            if (dynamicLibraryError != null) {
                throw new Exception("The engine core library failed to load.", dynamicLibraryError);
            }
        }

        /// <summary>
        /// Retrieves the raw <see cref="IEngine" /> object.
        /// </summary>
        /// <returns>The raw <see cref="IEngine" /> object.</returns>
        public static IEngine NativeEngine {
            get {
                EnsureLoaded();
                return nativeEngine;
            }
        }
    }
}