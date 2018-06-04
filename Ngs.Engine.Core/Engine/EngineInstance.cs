//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Linq;
using System.IO;
using System.Runtime.InteropServices;
using System.Xml.Serialization;
using System.Security;
using Ngs.Interop;
using Ngs.Utils;
using Ngs.Engine.Native;

namespace Ngs.Engine {
    delegate int NgsEngineCreate(out IntPtr retval);

    /// <summary>
    /// Provides an entry point to the game engine core.
    /// </summary>
    /// <remarks>
    /// <para>This class maintains a static reference to the game engine core. The initialization
    /// happens when a member of this class is used (explicitly or implicitly via wrapper classes)
    /// for the first time.</para>
    /// <para>The engine loader configuration file (refer to <see cref="LoaderConfigFileName" />
    /// for its name) is used to discover the engine core image file (<c>libngsengine</c>)
    /// during the initialization process.
    /// The configuration file is loaded from the engine path, which is determined using one of the
    /// following methods, with the later one taking precedence:</para>
    /// <list type="number">
    ///     <item><term>
    ///     The directory where this assembly is located.
    ///     </term></item>
    ///     <item><term>
    ///     The path specified by the environment variable <c>NGS_ENGINE_PATH</c>.
    ///     </term></item>
    /// </list>
    /// <para>The class <see cref="EngineLoaderConfig" /> defines the schema of the engine loader
    /// configuration file. See its documentation for details.</para>
    /// <para>Aside from the core library, a helper library named <c>libngsloader</c> is required
    /// during the initialization. This library must be located in the engine path.</para>
    /// <para>Note that this initialization process happens automatically as a part of static
    /// initialization of this class. There currently is no way for the application to control this
    /// behavior.</para>
    /// <para>(The design of the initialization process ia provisional and is subject to change.)</para>
    /// </remarks>
    public static class EngineInstance {
        static DynamicLibrary dynamicLibrary;
        static INgsEngine nativeEngine;
        static Exception loadError;

        /// <summary>
        /// Retrieves the name of the engine loader configuration file
        /// (<c>NgsLoaderConfig.xml</c>).
        /// </summary>
        public static readonly string LoaderConfigFileName = "NgsLoaderConfig.xml";

        static EngineInstance() {
            try {
                // Load the loader config
                string enginePath = EngineLoaderHelper.EnginePath;
                string loaderConfigPath = Path.Combine(enginePath, LoaderConfigFileName);

                if (!File.Exists(loaderConfigPath)) {
                    throw new Exception($"Could not locate the engine loader configuration file ({LoaderConfigFileName}).");
                }

                EngineLoaderConfig loaderConfig;
                using (var stream = File.OpenRead(loaderConfigPath)) {
                    var serializer = new XmlSerializer(typeof(EngineLoaderConfig));
                    loaderConfig = (EngineLoaderConfig)serializer.Deserialize(stream);
                }

                // Get the processor capability
                var processorInfo = EngineLoaderHelper.ProcessorInfo;

                // Choose the engine image
                string imagePath = null;
                foreach (var imageConfig in loaderConfig.Images) {
                    if (imageConfig.SupportedPlatforms == null) {
                        throw new Exception("Invalid loader configuration file: Must support at least one platform.");
                    }

                    if (imageConfig.Path == null) {
                        throw new Exception("Invalid loader configuration file: Path cannot be null.");
                    }

                    bool processorCompatible = imageConfig.RequiredProcessorFeatures?
                        .All(processorInfo.SupportsFeature) ?? true;

                    bool platformCompatible = imageConfig.SupportedPlatforms
                        .Any(name => {
                            switch (name) {
                                case "Windows":
                                    return RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
                                case "OSX":
                                    return RuntimeInformation.IsOSPlatform(OSPlatform.OSX);
                                case "Linux":
                                    return RuntimeInformation.IsOSPlatform(OSPlatform.Linux);
                                default:
                                    return false;
                            }
                        });

                    var thisPath = Path.Combine(enginePath, imageConfig.Path);

                    if (processorCompatible && platformCompatible && File.Exists(thisPath)) {
                        imagePath = thisPath;
                        break;
                    }
                }

                if (imagePath == null) {
                    throw new Exception("Could not find a engine image compatible with the current platform and processor.");
                }

                var library = DynamicLibrary.Load(imagePath);

                var entryPtr = library.GetSymbol("ngsengine_create");
                if (entryPtr == IntPtr.Zero) {
                    throw new Exception("Could not find the engine entry point.");
                }

                var entryDelegate = Marshal.GetDelegateForFunctionPointer<NgsEngineCreate>(entryPtr);
                Marshal.ThrowExceptionForHR(entryDelegate(out var enginePtr));

                nativeEngine = NgscomMarshal.GetRcwForInterfacePtr<INgsEngine>(enginePtr, false);
                dynamicLibrary = library;
            } catch (Exception ex) {
                loadError = ex;
            }
        }

        /// <summary>
        /// Ensures the game engine is loaded.
        /// </summary>
        /// <remarks>
        /// This method is called internally whenever an access to the engine's feature is
        /// made. Call this method explicitly to catch the exception thrown during the engine
        /// initialization at an intended timing.
        /// </remarks>
        /// <exception cref="EngineLoaderException">
        /// Thrown if there was an error while loading the game engine DLL.
        /// </exception>
        public static void EnsureLoaded() {
            if (loadError != null) {
                throw new EngineLoaderException("The engine core library failed to load.", loadError);
            }
        }

        /// <summary>
        /// Retrieves the raw <see cref="INgsEngine" /> object.
        /// </summary>
        /// <returns>The raw <see cref="INgsEngine" /> object.</returns>
        public static INgsEngine NativeEngine {
            [SecurityCritical]
            get {
                EnsureLoaded();
                return nativeEngine;
            }
        }
    }
}