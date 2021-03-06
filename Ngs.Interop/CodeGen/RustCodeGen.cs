//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
namespace Ngs.Interop.CodeGen {
    public sealed class RustCodeGen {
        private RustCodeGenOptions options;

        public RustCodeGen(RustCodeGenOptions options) {
            if (options == null) {
                throw new ArgumentNullException(nameof(options));
            }

            this.options = options;
        }

        private System.Text.StringBuilder stringBuilder;
        private HashSet<Type> generatedTypes;
        private Queue<Type> typeGenerationQueue;

        public string GenerateInterfaceDefinitions(IEnumerable<Type> types) {
            stringBuilder = new System.Text.StringBuilder();
            generatedTypes = new HashSet<Type>() {
                // built-in/special types
                typeof (Ngs.Interop.IUnknown),
                typeof (string)
            };
            typeGenerationQueue = new Queue<Type>();

            foreach (var t in options.TypeMapping.Keys) {
                generatedTypes.Add(t);
            }

            stringBuilder.AppendLine($"/* generated by {GetType().FullName} */");

            foreach (var type in types) {
                EnqueueTypeGeneration(type);
            }

            while (typeGenerationQueue.Count > 0) {
                GenerateType(typeGenerationQueue.Dequeue());
            }

            return stringBuilder.ToString().Replace("\r\n", "\n");
        }

        void EnqueueTypeGeneration(Type type) {
            if (type.IsConstructedGenericType) {
                throw new NotSupportedException($"Cannot emit a Rust interop code for the constructed generic type {type.FullName}.");
            }
            if (type.IsPointer || type.IsByRef) {
                EnqueueTypeGeneration(type.GetElementType());
                return;
            }
            if (generatedTypes.Contains(type)) {
                return;
            }
            generatedTypes.Add(type);
            typeGenerationQueue.Enqueue(type);
        }

        void GenerateType(Type type) {
            if (options.TypeMapping.ContainsKey(type)) {
                return;
            }

            stringBuilder.AppendLine($"/* {type.FullName} */");

            var typeInfo = type.GetTypeInfo();
            if (typeInfo.IsInterface) {
                GenerateInterface(type);
            } else if (typeInfo.IsSubclassOf(typeof(Enum))) {
                GenerateEnum(type);
            } else if (typeInfo.IsValueType) {
                GenerateStruct(type);
            } else {
                throw new NotSupportedException($"Cannot emit a Rust interop code for the type {typeInfo.FullName}.");
            }
        }

        static string ByteArrayToIntegerConstant(byte[] array, int start, int length, bool bytewise) {
            var sb = new System.Text.StringBuilder();
            if (!bytewise) {
                sb.Append("0x");
            }
            for (int i = 0; i < length; ++i) {
                if (bytewise) {
                    if (i > 0) {
                        sb.Append(", ");
                    }
                    sb.Append("0x");
                }
                sb.Append(array[i + start].ToString("X2"));
            }
            return sb.ToString();
        }

        sealed class RustParameterInfo {
            public RustCodeGen RustCodeGen { get; }
            public Marshaller.ComMethodParameterInfo ComMethodParameterInfo { get; }
            public string NativeName { get; }

            public RustParameterInfo(RustCodeGen gen, Marshaller.ComMethodParameterInfo cmpi) {
                RustCodeGen = gen;
                ComMethodParameterInfo = cmpi;

                gen.EnqueueTypeGeneration(cmpi.Type);
                NativeName = ComMethodParameterInfo.IsReturnValue ? "retval" :
                    SnakeCaseConverter.Join(DotNetCamelCaseConverter.Split(ComMethodParameterInfo.ParameterInfo.Name)).ToLowerInvariant();
            }

            public string NativeTypeName => RustCodeGen.TranslateNativeType(ComMethodParameterInfo.NativeType, !ComMethodParameterInfo.IsOut, ComMethodParameterInfo.IsByRef);
        }

        sealed class RustInterfaceMethodInfo {
            public RustCodeGen RustCodeGen { get; }
            public Marshaller.ComMethodInfo ComMethodInfo { get; }
            public RustParameterInfo[] ParameterInfos { get; }
            public string NativeName { get; }
            public string NativeReturnTypeName { get; }
            public RustdocEntry? RustdocEntry { get; }

            public RustInterfaceMethodInfo(RustCodeGen gen, Marshaller.ComMethodInfo cmi) {
                RustCodeGen = gen;
                ComMethodInfo = cmi;
                ParameterInfos = cmi.ParameterInfos.Select((pi) => new RustParameterInfo(gen, pi)).ToArray();

                string name = cmi.MethodInfo.Name;
                if (name.StartsWith("get_") || name.StartsWith("set_")) {
                    NativeName = name.Substring(0, 4) + SnakeCaseConverter.Join(DotNetPascalCaseConverter.Split(name.Substring(4))).ToLowerInvariant();
                } else {
                    NativeName = SnakeCaseConverter.Join(DotNetPascalCaseConverter.Split(name)).ToLowerInvariant();
                }

                // Retrieve the documentation entry
                if (gen.options.RustdocEntrySource != null) {
                    if (name.StartsWith("get_") || name.StartsWith("set_")) {
                        var prop = cmi.MethodInfo.DeclaringType.GetProperty(name.Substring(4));
                        if (prop != null) {
                            RustdocEntry = gen.options.RustdocEntrySource.GetEntryForProperty(prop,
                                name.StartsWith("set_"));
                        }
                    } else {
                        RustdocEntry = gen.options.RustdocEntrySource.GetEntryForMethod(cmi.MethodInfo);
                    }
                }

                if (cmi.ReturnsHresult) {
                    NativeReturnTypeName = $"{gen.options.NgscomCratePath}::HResult";
                } else {
                    NativeReturnTypeName = RustCodeGen.TranslateNativeType(cmi.NativeReturnType, false, false);
                }
            }

            public string GetSignature(bool isInterfaceDeclaration) {
                var sb = new System.Text.StringBuilder();
                sb.Append($"fn {NativeName}(");

                var paramDecl = ParameterInfos.Select((rpi) => {
                    return $"{rpi.NativeName}: {rpi.NativeTypeName}";
                }).ToList();
                if (!isInterfaceDeclaration) {
                    paramDecl.Insert(0, "&self");
                }

                sb.Append(string.Join(", ", paramDecl));
                sb.Append($") -> {NativeReturnTypeName}");
                return sb.ToString();
            }
        }

        void GenerateInterface(Type type) {
            var info = new Marshaller.InterfaceInfo(type);
            var typeInfo = type.GetTypeInfo();

            var name = type.Name;
            if (!name.StartsWith("I")) {
                throw new NotSupportedException($"The name of interface {type.FullName} doesn't start with the character 'I'.");
            }
            var nameDecomposed = DotNetPascalCaseConverter.Split(name.Substring(1));
            nameDecomposed[0] = "I" + nameDecomposed[0];
            var upperSnakeCaseName = SnakeCaseConverter.Join(nameDecomposed).ToUpperInvariant();
            var lowerSnakeCaseName = SnakeCaseConverter.Join(nameDecomposed).ToLowerInvariant();

            var iidIdt = "IID_" + upperSnakeCaseName;
            var iid = info.ComGuid.ToByteArray();
            stringBuilder.AppendLine($"com_iid!({iidIdt} = [{ByteArrayToIntegerConstant(iid, 0, 4, false)}, " +
                $"{ByteArrayToIntegerConstant(iid, 4, 2, false)}, {ByteArrayToIntegerConstant(iid, 6, 2, false)}, " +
                $"[{ByteArrayToIntegerConstant(iid, 8, 8, true)}]]);");

            var baseTypes = new List<Type>();
            var indirectlyBaseTypes = new HashSet<Type>();
            Action<Type> scanIndirectBase = null;
            scanIndirectBase = (theType) => {
                indirectlyBaseTypes.Add(theType);
                foreach (var bt in theType.GetTypeInfo().ImplementedInterfaces) {
                    scanIndirectBase(bt);
                }
            };
            foreach (var bt in typeInfo.ImplementedInterfaces) {
                baseTypes.Add(bt);

                var ti2 = bt.GetTypeInfo();
                if (!ti2.IsInterface) {
                    throw new InvalidOperationException();
                }

                foreach (var bt2 in ti2.ImplementedInterfaces) {
                    scanIndirectBase(bt2);
                }
            }

            var realBaseTypes = baseTypes.Where((t) => !indirectlyBaseTypes.Contains(t)).ToArray();
            if (realBaseTypes.Length > 1) {
                throw new NotSupportedException($"Cannot emit interop code for the interface {type.FullName} because " +
                    "Interfaces with multiple base interface are not supported by NgsCOM.");
            }

            foreach (var t in realBaseTypes) {
                EnqueueTypeGeneration(t);
            }

            var baseDeclarations = new List<string>();

            foreach (var t in realBaseTypes) {
                var nativeName = GetOutputIdentifier(t);
                baseDeclarations.Add($"({nativeName}, {nativeName}Trait)");
            }
            foreach (var t in indirectlyBaseTypes) {
                var nativeName = GetOutputIdentifier(t);
                baseDeclarations.Add(nativeName);
            }

            stringBuilder.AppendLine("com_interface! {");

            var methods = info.ComMethodInfos
                .Where((cmi) => cmi.MethodInfo.DeclaringType == info.Type)
                .Select((cmi) => new RustInterfaceMethodInfo(this, cmi)).ToArray();

            // Emit the documentation comment
            {
                var doc = options.RustdocEntrySource?.GetEntryForType(type);
                if (doc.HasValue) {
                    GenerateDocComment(doc, "\t");
                } else {
                    stringBuilder.AppendLine($"\t/// `{type.FullName}`");
                }
                stringBuilder.AppendLine("\t///");
                stringBuilder.AppendLine("\t/// # COM interop");
                stringBuilder.AppendLine($"\t/// This type was generated automatically from `{type.FullName}`.");

                // Emit the template code here
                stringBuilder.AppendLine("\t///");
                stringBuilder.AppendLine($"\t/// Use the following template code to create a COM class");
                stringBuilder.AppendLine($"\t/// that implements `{name}`:");
                stringBuilder.AppendLine("\t///");
                stringBuilder.AppendLine("\t/// ```ignore");
                stringBuilder.AppendLine("\t/// com_impl! {");
                stringBuilder.AppendLine("\t///     class MyClassName {");
                stringBuilder.AppendLine($"\t///         {lowerSnakeCaseName}: {name};");
                stringBuilder.AppendLine("\t///         @data: MyClassNameData;");
                stringBuilder.AppendLine("\t///     }");
                stringBuilder.AppendLine("\t/// }");
                stringBuilder.AppendLine("\t///");
                stringBuilder.AppendLine($"\t/// impl {name}Trait for MyClassName {{");
                stringBuilder.AppendLine("\t///");
                foreach (var rimi in methods) {
                    stringBuilder.AppendLine($"\t///     {rimi.GetSignature(false)} {{");
                    if (rimi.ComMethodInfo.ReturnsHresult) {
                        stringBuilder.AppendLine("\t///         \thresults::E_NOTIMPL");
                    } else {
                        stringBuilder.AppendLine("\t///         \tunimplemented!()");
                    }
                    stringBuilder.AppendLine("\t///     }");
                    stringBuilder.AppendLine("\t///");
                }
                stringBuilder.AppendLine("\t/// }");
                stringBuilder.AppendLine("\t/// ```");
                stringBuilder.AppendLine("\t///");
            }

            stringBuilder.AppendLine($"\tinterface ({name}, {name}Trait): {string.Join(", ", baseDeclarations)} {{");
            stringBuilder.AppendLine($"\t\tiid: {iidIdt},");
            stringBuilder.AppendLine($"\t\tvtable: {name}Vtbl,");
            stringBuilder.AppendLine();

            foreach (var rimi in methods) {
                GenerateDocComment(rimi.RustdocEntry, "\t\t");

                stringBuilder.AppendLine($"\t\t{rimi.GetSignature(true)};");
            }

            stringBuilder.AppendLine("\t}");
            stringBuilder.AppendLine("}");
            stringBuilder.AppendLine();
        }

        static readonly Dictionary<Type, string> structUnderlyingTypeMap = new Dictionary<Type, string>
        {
            [typeof(sbyte)] = "u8",
            [typeof(short)] = "u16",
            [typeof(int)] = "u32",
            [typeof(long)] = "u64",
            [typeof(byte)] = "u8",
            [typeof(ushort)] = "u16",
            [typeof(uint)] = "u32",
            [typeof(ulong)] = "u64"
        };

        void GenerateStruct(Type type) {
            var doc = options.RustdocEntrySource?.GetEntryForType(type);
            if (doc.HasValue) {
                GenerateDocComment(doc, "");
            } else {
                stringBuilder.AppendLine($"/// `{type.FullName}`");
            }

            stringBuilder.AppendLine("///");
            stringBuilder.AppendLine("/// # COM interop");
            stringBuilder.AppendLine($"/// This type was generated automatically from `{type.FullName}`.");

            stringBuilder.AppendLine("#[repr(C)]");
            stringBuilder.AppendLine("#[derive(Debug, Clone, Copy)]");
            stringBuilder.AppendLine($"pub struct {type.Name} {{");
            foreach (var field in type.GetRuntimeFields()) {
                doc = options.RustdocEntrySource?.GetEntryForField(field);
                GenerateDocComment(doc, "\t");

                var nativeName = SnakeCaseConverter.Join(DotNetCamelCaseConverter.Split(field.Name));
                var nativeType = TranslateNativeType(field.FieldType, false, false);
                stringBuilder.AppendLine($"\tpub {nativeName}: {nativeType},");
            }
            stringBuilder.AppendLine("}");
            stringBuilder.AppendLine();
        }

        void GenerateEnum(Type type) {
            var ut = structUnderlyingTypeMap[Enum.GetUnderlyingType(type)];
            bool isFlags = type.GetCustomAttribute(typeof(FlagsAttribute)) != null;

            if (isFlags) {
                GenerateBitflags(type);
                return;
            }

            stringBuilder.AppendLine($"/// Valid values for `{type.Name}`.");
            stringBuilder.AppendLine("///");
            stringBuilder.AppendLine("/// # COM interop");
            stringBuilder.AppendLine($"/// This type was generated automatically from `{type.FullName}`.");
            stringBuilder.AppendLine($"#[repr({ut})]");
            stringBuilder.AppendLine("#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]");
            stringBuilder.AppendLine($"pub enum {type.Name}Item {{");
            foreach (object field in Enum.GetValues(type)) {
                var name = Enum.GetName(type, field);
                var value = Convert.ToInt32(field);

                var fdoc = options.RustdocEntrySource?.GetEntryForField(type.GetField(name));
                GenerateDocComment(fdoc, "\t");

                stringBuilder.AppendLine($"\t{name} = {value},");
            }
            stringBuilder.AppendLine("}");
            stringBuilder.AppendLine();

            var doc = options.RustdocEntrySource?.GetEntryForType(type);
            if (doc.HasValue) {
                GenerateDocComment(doc, "");
            } else {
                stringBuilder.AppendLine($"/// `{type.FullName}`");
            }

            stringBuilder.AppendLine("///");
            stringBuilder.AppendLine("/// # COM interop");
            stringBuilder.AppendLine($"/// This type was generated automatically from `{type.FullName}`.");

            stringBuilder.AppendLine("#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]");
            stringBuilder.AppendLine($"pub struct {type.Name}(pub {ut});");
            stringBuilder.AppendLine($"impl {type.Name} {{");
            stringBuilder.AppendLine($"\t/// Return a value of `{type.Name}Item` if it contains a valid value of `{type.Name}Item`,");
            stringBuilder.AppendLine($"\t/// or `None` otherwise.");
            stringBuilder.AppendLine($"\tpub fn get(&self) -> Option<{type.Name}Item> {{");
            foreach (object field in Enum.GetValues(type)) {
                var name = Enum.GetName(type, field);
                var value = Convert.ToInt32(field);
                stringBuilder.AppendLine($"\t\tif self.0 == {value} {{");
                stringBuilder.AppendLine($"\t\t\treturn Some({type.Name}Item::{name});");
                stringBuilder.AppendLine("\t\t}");
            }
            stringBuilder.AppendLine("\t\tNone");
            stringBuilder.AppendLine("\t}");
            stringBuilder.AppendLine("}");
            stringBuilder.AppendLine();
            stringBuilder.AppendLine($"impl From<{type.Name}Item> for {type.Name} {{");
            stringBuilder.AppendLine($"\tfn from(x: {type.Name}Item) -> Self {{");
            stringBuilder.AppendLine($"\t\t{type.Name}(x as {ut})");
            stringBuilder.AppendLine("\t}");
            stringBuilder.AppendLine("}");
            stringBuilder.AppendLine();
        }

        /// Called by <see cref="GenerateEnum" /> when <paramref name="type" /> has
        /// <see cref="FlagsAttribute" />.
        void GenerateBitflags(Type type) {
            var ut = structUnderlyingTypeMap[Enum.GetUnderlyingType(type)];
            // TODO
            stringBuilder.AppendLine($"{options.BitflagsMacroPath}! {{");

            var doc = options.RustdocEntrySource?.GetEntryForType(type);
            if (doc.HasValue) {
                GenerateDocComment(doc, "\t");
            } else {
                stringBuilder.AppendLine($"\t/// `{type.FullName}`");
            }

            stringBuilder.AppendLine("\t///");
            stringBuilder.AppendLine("\t/// # COM interop");
            stringBuilder.AppendLine($"\t/// This type was generated automatically from `{type.FullName}`.");

            stringBuilder.AppendLine($"\tpub struct {type.Name}: {ut} {{");
            foreach (object field in Enum.GetValues(type)) {
                var name = Enum.GetName(type, field);
                var value = Convert.ToInt32(field);

                var fdoc = options.RustdocEntrySource?.GetEntryForField(type.GetField(name));
                GenerateDocComment(fdoc, "\t\t");

                stringBuilder.AppendLine($"\t\tconst {name} = {value};");
            }
            stringBuilder.AppendLine("\t}");
            stringBuilder.AppendLine("}");
            stringBuilder.AppendLine();
        }

        void GenerateDocComment(RustdocEntry? ent, string indent) {
            if (!ent.HasValue) {
                return;
            }
            foreach (var line in ent.Value.Text.Trim().Split('\n')) {
                stringBuilder.AppendLine(indent + "/// " + line);
            }
        }

        string TranslateNativeType(Type type, bool isConstReference, bool isByRefParam) {
            if (type == typeof(string)) {
                return $"Option<&{options.NgscomCratePath}::BString>";
            } else if (type.IsPointer && !isByRefParam) {
                return "*mut " + TranslateNativeType(type.GetElementType(), false, false);
            } else if (type.IsByRef || (type.IsPointer && isByRefParam)) {
                var elem = type.GetElementType();
                string nativeElem;
                if (elem == typeof(string)) {
                    nativeElem = $"{options.NgscomCratePath}::BStringRef";
                } else if (elem.GetTypeInfo().IsInterface) {
                    nativeElem = $"{options.NgscomCratePath}::ComPtr<" + GetOutputIdentifier(elem) + ">";
                } else {
                    nativeElem = TranslateNativeType(elem, false, false);
                }
                if (isConstReference) {
                    return "&" + nativeElem;
                } else {
                    return "&mut " + nativeElem;
                }
            } else if (type.GetTypeInfo().IsInterface) {
                return $"{options.NgscomCratePath}::UnownedComPtr<" + GetOutputIdentifier(type) + ">";
            } else {
                return GetOutputIdentifier(type);
            }
        }

        string GetOutputIdentifier(Type type) {
            if (type == typeof(IUnknown)) {
                return $"{options.NgscomCratePath}::IUnknown";
            }
            string idt;
            if (options.TypeMapping.TryGetValue(type, out idt)) {
                return idt;
            }
            return type.Name;
        }
    }
}