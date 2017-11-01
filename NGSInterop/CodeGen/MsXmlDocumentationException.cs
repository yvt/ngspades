//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Reflection;
using System.Xml;
using System.Collections.Generic;
using System.IO;
namespace Ngs.Interop.CodeGen
{
    /// <summary>
    /// An exception thrown when an error occured while reading a Microsoft
    /// XML documentation file.
    /// </summary>
    public class MsXmlDocumentationException : Exception
    {
        /// <summary>
        /// Initializes a new instance of the <see cref="MsXmlDocumentationException" />
        /// class with a specified error message and the exception that is the
        /// cause of this exception.
        /// </summary>
        /// <param name="message">The error message.</param>
        /// <param name="innerException">
        /// The exception that is the cause of the current exception.
        /// </param>
        public MsXmlDocumentationException(string message, Exception innerException)
        : base(message, innerException)
        {
        }
    }
}