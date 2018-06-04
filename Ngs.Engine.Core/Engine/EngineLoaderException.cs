//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Runtime.Serialization;

namespace Ngs.Engine {
    /// <summary>
    /// The exception that is thrown when something goes wrong in the engine
    /// loader.
    /// </summary>
    public class EngineLoaderException : Exception {
        /// <summary>
        /// Initializes a new instance of <see cref="EngineLoaderException" />
        /// using the default message.
        /// </summary>
        public EngineLoaderException() : base("Failed to load the engine core subsystem.") {
        }

        /// <summary>
        /// Initializes a new instance of <see cref="EngineLoaderException" />
        /// with its <see cref="Exception.Message" /> set to <paramref name="message" />.
        /// </summary>
        public EngineLoaderException(string message) : base(message) {
        }

        /// <summary>
        /// Initializes a new instance of <see cref="EngineLoaderException" />
        /// with its <see cref="Exception.Message" /> set to <paramref name="message" />
        /// and its <see cref="Exception.InnerException" /> set to <paramref name="innerException" />.
        /// </summary>
        public EngineLoaderException(string message, Exception innerException) : base(message, innerException) {
        }
    }
}
