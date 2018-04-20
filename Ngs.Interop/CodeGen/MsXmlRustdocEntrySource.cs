//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Reflection;
using System.Text;
using System.Xml;
namespace Ngs.Interop.CodeGen {
    /// <summary>
    /// Generates rustdoc documentation entries from on Microsoft XML Documentation
    /// files.
    /// </summary>
    public sealed class MsXmlRustdocEntrySource : IRustdocEntrySource {
        MsXmlDocumentationReader reader;

        /// <summary>
        /// Initializes a new instance of the <see cref="MsXmlRustdocEntrySource" />
        /// class.
        /// </summary>
        /// <param name="reader">
        /// The <see cref="MsXmlDocumentationReader" /> to read documentations
        /// from.
        /// </param>
        public MsXmlRustdocEntrySource(MsXmlDocumentationReader reader) {
            if (reader == null) {
                throw new ArgumentNullException(nameof(reader));
            }
            this.reader = reader;
        }

        sealed class Converter {
            StringBuilder output = new StringBuilder();
            StringBuilder para = new StringBuilder();
            StringBuilder links = new StringBuilder();

            void NewParagraph() {
                var s = para.ToString().Trim();
                para.Clear();
                if (s.Length > 0) {
                    if (output.Length > 0) {
                        output.Append("\n\n");
                    }
                    output.Append(s);
                }
            }

            void ProcessBulletList(XmlElement root) {
                throw new NotImplementedException();
            }

            void ScanParagraph(XmlElement root) {
                NewParagraph();
                foreach (var child in root.ChildNodes) {
                    switch (child) {
                        case XmlText text:
                            var txt = text.Value;
                            txt = txt.Replace("\r\n", "\n");
                            txt = txt.Replace("\r", "\n");
                            var lines = text.Value.Split('\n');
                            for (int i = 0; i < lines.Length; i += 1) {
                                para.Append(lines[i].Trim());
                                if (i == lines.Length - 1) {
                                    para.Append(" ");
                                } else {
                                    para.AppendLine();
                                }
                            }
                            break;
                        case XmlElement e:
                            if (e.Name == "c") {
                                para.Append($"`{e.InnerText}`");
                            } else if (e.Name == "see") {
                                var cref = e.GetAttribute("cref");
                                var href = e.GetAttribute("href");
                                string linkTarget = null;
                                string text = "";

                                if (cref != null) {
                                    // Remove the namespace part and convert the name
                                    // to the Rust format
                                    var parts = cref.Split('.');
                                    var baseName = parts[parts.Length - 1];
                                    var type = cref.Substring(0, 2);

                                    if (type == "F:") {
                                        // TODO: disambiguate between enum field and struct field?
                                    } else if (type == "T:") {
                                        linkTarget = $"struct.{baseName}.html";
                                    }
                                    text = $"`{baseName}`";
                                } else if (href != null) {
                                    linkTarget = href;
                                }

                                if (e.InnerText != "") {
                                    text = e.InnerText;
                                }

                                if (linkTarget != null) {
                                    links.AppendLine($"[{text}]: {linkTarget}");
                                    para.Append($"[{text}] ");
                                } else {
                                    para.Append(text);
                                }
                            } else if (e.Name == "summary" || e.Name == "remarks" || e.Name == "para") {
                                ScanParagraph(e);
                            } else if (e.Name == "param" || e.Name == "typeparam" || e.Name == "returns") {
                                // Ignore for now
                            } else if (e.Name == "paramref" || e.Name == "typeparamref") {
                                var name = e.GetAttribute("name");
                                para.Append($"`{name}`");
                            } else if (e.Name == "list") {
                                var type = e.GetAttribute("type");
                                if (type == "bullet") {
                                    ProcessBulletList(e);
                                }
                            } else {
                                // Unrecognized inline element
                                para.Append(e.InnerText);
                            }
                            break;
                    }
                }
                NewParagraph();
            }

            public string Convert(XmlElement root) {
                ScanParagraph(root);
                if (links.Length > 0) {
                    output.AppendLine();
                    output.AppendLine();
                    output.Append(links.ToString());
                }
                return output.ToString();
            }
        }

        RustdocEntry ConvertToRustdoc(XmlElement root) {
            var c = new Converter();
            return new RustdocEntry(c.Convert(root));
        }

        /// <summary>
        /// Retrieves a documentation entry for the specified type.
        /// </summary>
        /// <param name="t">The type to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        public RustdocEntry? GetEntryForType(Type t) {
            var e = reader.GetEntryForType(t);
            if (e != null) {
                return ConvertToRustdoc(e);
            } else {
                return null;
            }
        }

        /// <summary>
        /// Retrieves a documentation entry for the specified method.
        /// </summary>
        /// <param name="method">The method to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        public RustdocEntry? GetEntryForMethod(MethodInfo method) {
            var e = reader.GetEntryForMethod(method);
            if (e != null) {
                return ConvertToRustdoc(e);
            } else {
                return null;
            }
        }

        /// <summary>
        /// Retrieves a documentation entry for the specified field.
        /// </summary>
        /// <param name="field">The field to retrieve a documentation for.</param>
        /// <returns>The documentation entry if any.</returns>
        public RustdocEntry? GetEntryForField(FieldInfo field) {
            var e = reader.GetEntryForField(field);
            if (e != null) {
                return ConvertToRustdoc(e);
            } else {
                return null;
            }
        }

        /// <summary>
        /// Retrieves a documentation entry for the specified property.
        /// </summary>
        /// <param name="prop">The property to retrieve a documentation for.</param>
        /// <param name="setter">
        /// <c>true</c> to retrieve the entry for the setter.
        /// </param>
        /// <returns>The documentation entry if any.</returns>
        public RustdocEntry? GetEntryForProperty(PropertyInfo prop, bool setter) {
            var e = reader.GetEntryForProperty(prop);
            if (e != null) {
                var d = ConvertToRustdoc(e);
                if (setter) {
                    d = new RustdocEntry($"Set the value of the `{prop.Name}` property.\n\n" + d.Text);
                } else {
                    d = new RustdocEntry($"Retrieve the value of the `{prop.Name}` property.\n\n" + d.Text);
                }
                return d;
            } else {
                return null;
            }
        }
    }
}