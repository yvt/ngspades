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
using System.Linq;
namespace Ngs.Interop.CodeGen
{
    /// <summary>
    /// Fetches documentation entries from Microsoft XML Documentation files
    /// (as produced by <c>csc /doc</c>).
    /// </summary>
    /// <remarks>
    /// Methods from this class throw the <see cref="MsXmlDocumentationException" />
    /// exception when they encounter an error while processing a Microsoft
    /// XML Documentation file.
    /// </remarks>
    public sealed class MsXmlDocumentationReader
    {
        /// <summary>
        /// A map from member identifiers to the corresponding XML elements.
        /// </summary>
        Dictionary<string, XmlElement> map = new Dictionary<string, XmlElement>();

        /// <summary>
        /// A set of the file names of the already imported XML files.
        /// </summary>
        HashSet<string> loadedFiles = new HashSet<string>();

        /// <summary>
        /// Creates a <see cref="MsXmlDocumentationReader" />.
        /// </summary>
        public MsXmlDocumentationReader()
        {
        }

        void Import(XmlDocument doc, string fileNameOrNull)
        {
            try
            {
                var members = doc.SelectNodes("/doc/members/member");

                foreach (XmlElement member in members)
                {
                    var name = member.GetAttribute("name");
                    if (name == null)
                    {
                        continue;
                    }

                    if (!map.ContainsKey(name))
                    {
                        map[name] = member;
                    }
                }
            }
            catch (Exception ex)
            {
                throw new MsXmlDocumentationException(
                    fileNameOrNull != null ?
                        "An error occured while reading the Microsoft XML " +
                        $"documentation file '{fileNameOrNull}'." :
                        "An error occured while reading a Microsoft XML " +
                        "documentation file.",
                    ex
                );
            }
        }

        bool TryLoadAssemblyDocumentation(Assembly assembly)
        {
            string fileName = assembly.Location;
            if (fileName.Length == 0)
            {
                // Cannot load the XML documentation because the assembly does
                // exist on a disk.
                return false;
            }

            fileName = Path.Combine(
                Path.GetDirectoryName(fileName),
                Path.GetFileNameWithoutExtension(fileName) + ".xml"
            );

            if (loadedFiles.Contains(fileName))
            {
                // The documentation is already imported.
                return false;
            }

            loadedFiles.Add(fileName);

            if (!File.Exists(fileName))
            {
                // The documentation file does not exist.
                return false;
            }

            var doc = new XmlDocument();
            doc.Load(fileName);
            Import(doc, fileName);

            return true;
        }

        /// <summary>
        /// Retrieves a documentation entry for the specified type.
        /// </summary>
        /// <param name="type">The type to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        public XmlElement GetEntryForType(Type type)
        {
            string idt = "T:" + type.FullName;
            if (map.TryGetValue(idt, out var e))
            {
                return e;
            }
            if (!TryLoadAssemblyDocumentation(type.Assembly))
            {
                return null;
            }
            map.TryGetValue(idt, out e);
            return e;
        }

        /// <summary>
        /// Retrieves a documentation entry for the specified method.
        /// </summary>
        /// <param name="method">The method to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        public XmlElement GetEntryForMethod(MethodInfo method)
        {
            string idt = "M:" + method.DeclaringType.FullName + "." + method.Name;
            if (method.GetParameters().Length > 0)
            {
                idt += "(" + string.Join(",", method.GetParameters()
                    .Select((p) => p.ParameterType.FullName)) + ")";
            }
            if (map.TryGetValue(idt, out var e))
            {
                return e;
            }
            if (!TryLoadAssemblyDocumentation(method.DeclaringType.Assembly))
            {
                return null;
            }
            map.TryGetValue(idt, out e);
            return e;
        }

        /// <summary>
        /// Retrieves a documentation entry for the specified field.
        /// </summary>
        /// <param name="field">The field to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        public XmlElement GetEntryForField(FieldInfo field)
        {
            string idt = "F:" + field.DeclaringType.FullName + "." + field.Name;
            if (map.TryGetValue(idt, out var e))
            {
                return e;
            }
            if (!TryLoadAssemblyDocumentation(field.DeclaringType.Assembly))
            {
                return null;
            }
            map.TryGetValue(idt, out e);
            return e;
        }

        /// <summary>
        /// Retrieves a documentation entry for the specified property.
        /// </summary>
        /// <param name="prop">The property to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        public XmlElement GetEntryForProperty(PropertyInfo prop)
        {
            string idt = "P:" + prop.DeclaringType.FullName + "." + prop.Name;
            if (map.TryGetValue(idt, out var e))
            {
                return e;
            }
            if (!TryLoadAssemblyDocumentation(prop.DeclaringType.Assembly))
            {
                return null;
            }
            map.TryGetValue(idt, out e);
            return e;
        }
    }
}