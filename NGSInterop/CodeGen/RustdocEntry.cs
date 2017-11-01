//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
namespace Ngs.Interop.CodeGen
{
    /// <summary>
    /// A single documentation comment for rustdoc (a documentation generator
    /// for the Rust progamming language).
    /// </summary>
    public struct RustdocEntry
    {
        /// <summary>
        /// Retrieves the comment text.
        /// </summary>
        /// <returns>The comment text formatted in the Markdown format.</returns>
        public string Text { get; }

        /// <summary>
        /// Creates a <see cref="RustdocEntry" /> with the specified text.
        /// </summary>
        /// <param name="text">
        /// The comment text formatted in the Markdown format.
        /// </param>
        public RustdocEntry(string text)
        {
            Text = text;
        }
    }
}