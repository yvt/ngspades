//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Xml.Serialization;

namespace Ngs.Engine {
    /// <summary>
    /// The configuration for loading a engine core dynamic library.
    /// </summary>
    /// <remarks>
    /// <see cref="EngineInstance" /> automatically loads this object from the loader
    /// configuration file and uses it to discover the engine core image. See the documentation of
    /// <see cref="EngineInstance" /> for details on the loading process.
    /// </remarks>
    [XmlRoot("LoaderConfig", IsNullable = false)]
    public class EngineLoaderConfig {
        /// <summary>
        /// Sets or retrives the list of available engine images.
        /// </summary>
        [XmlElement(ElementName = "Image")]
        public EngineLoaderImageConfig[] Images { get; set; }
    }

    /// <summary>
    /// Describes a single engine image that can be loaded on a certain environment.
    /// </summary>
    public class EngineLoaderImageConfig {
        /// <summary>
        /// Sets or retrieves the list of processor features required to load this engine image.
        /// </summary>
        [XmlElement(ElementName = "RequiresProcessorFeature", IsNullable = false)]
        public string[] RequiredProcessorFeatures { get; set; }

        /// <summary>
        /// Sets or retrieves the list of platform supported by this engine image.
        /// </summary>
        [XmlElement(ElementName = "SupportsPlatform", IsNullable = false)]
        public string[] SupportedPlatforms { get; set; }

        /// <summary>
        /// Sets or retrieves the path to this engine image, relative to the loader configuration
        /// file.
        /// </summary>
        [XmlAttribute]
        public string Path { get; set; }
    }
}
