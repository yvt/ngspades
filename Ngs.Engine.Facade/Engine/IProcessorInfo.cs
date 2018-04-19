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
    /// <summary>
    /// Provides an information about the processor the application is running on.
    /// </summary>
    [Guid("af567d31-59c8-4c69-97a7-3586d72eecdd")]
    public interface IProcessorInfo : IUnknown {
        /// <summary>
        /// Retrieves the processor's vendor identification string.
        /// </summary>
        /// <returns>The processor's vendor identification string.</returns>
        string Vendor { get; }

        /// <summary>
        /// Returns whether a specified feature is supported by the processor.
        /// </summary>
        /// <param name="name">The name of the feature.</param>
        /// <returns>A flag indicating whether the feature is supported or not.</returns>
        bool SupportsFeature(string name);
    }
}