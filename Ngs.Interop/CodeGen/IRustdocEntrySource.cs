//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Reflection;
namespace Ngs.Interop.CodeGen {
    /// <summary>
    /// Provides rustdoc documentations for types and their members.
    /// </summary>
    public interface IRustdocEntrySource {
        /// <summary>
        /// Retrieves a documentation entry for the specified type.
        /// </summary>
        /// <param name="t">The type to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        RustdocEntry? GetEntryForType(Type t);

        /// <summary>
        /// Retrieves a documentation entry for the specified method.
        /// </summary>
        /// <param name="method">The method to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        RustdocEntry? GetEntryForMethod(MethodInfo method);

        /// <summary>
        /// Retrieves a documentation entry for the specified field.
        /// </summary>
        /// <param name="field">The field to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        RustdocEntry? GetEntryForField(FieldInfo field);

        /// <summary>
        /// Retrieves a documentation entry for the specified property.
        /// </summary>
        /// <param name="prop">The property to retrieve a documentation for.</param>
        /// <param name="setter">
        /// <c>true</c> to retrieve the entry for the setter.
        /// </param>
        /// <returns>The documentation entry if any.</returns>
        RustdocEntry? GetEntryForProperty(PropertyInfo prop, bool setter);
    }
}