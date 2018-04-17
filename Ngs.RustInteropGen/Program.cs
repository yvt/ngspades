//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Linq;
using System.Numerics;
using System.Reflection;
using Microsoft.Extensions.CommandLineUtils;

namespace Ngs.Interop.Shell {
    class RustInteropGen {
        static void Main(string[] args) {
            CommandLineApplication commandLineApplication = new CommandLineApplication(throwOnUnexpectedArg: false);
            CommandOption outputFile = commandLineApplication.Option(
                "-o |--output <FileName>",
                "The file name of Rust source code to which the output will be written.",
                CommandOptionType.SingleValue);
            commandLineApplication.HelpOption("-? | -h | --help");
            commandLineApplication.OnExecute(() => {
                var asm = typeof(Ngs.Engine.ITestInterface).GetTypeInfo().Assembly;
                var ifTypes = asm.GetTypes().Where((type) =>
                  type.GetTypeInfo().IsPublic &&
                  type.GetTypeInfo().IsInterface &&
                  typeof(Ngs.Interop.IUnknown).IsAssignableFrom(type));
                var options = new Ngs.Interop.CodeGen.RustCodeGenOptions();

                options.RustdocEntrySource = new Ngs.Interop.CodeGen.MsXmlRustdocEntrySource(
                    new Ngs.Interop.CodeGen.MsXmlDocumentationReader()
                );

                // Register geometry types.
                // Vector types in `Ngs.Utils` can represent both of displacements (`VectorX`) and
                // locations (`PointX`). We just pick the former as they can be converted
                // interchangeably.
                options.TypeMapping[typeof(Vector2)] = "::cgmath::Vector2<f32>";
                options.TypeMapping[typeof(Vector3)] = "::cgmath::Vector3<f32>";
                options.TypeMapping[typeof(Vector4)] = "::cgmath::Vector4<f32>";
                options.TypeMapping[typeof(Ngs.Utils.Matrix4)] = "::cgmath::Matrix4<f32>";
                options.TypeMapping[typeof(Ngs.Utils.IntVector2)] = "::cgmath::Vector2<i32>";
                options.TypeMapping[typeof(Ngs.Utils.IntVector3)] = "::cgmath::Vector3<i32>";
                options.TypeMapping[typeof(Ngs.Utils.IntVector4)] = "::cgmath::Vector4<i32>";
                options.TypeMapping[typeof(Ngs.Utils.Box2)] = "super::Box2<f32>";
                options.TypeMapping[typeof(Ngs.Utils.Box3)] = "super::Box3<f32>";

                var codegen = new Ngs.Interop.CodeGen.RustCodeGen(options);

                var code = codegen.GenerateInterfaceDefinitions(ifTypes);

                if (outputFile.HasValue()) {
                    System.IO.File.WriteAllText(outputFile.Value(), code);
                } else {
                    Console.Out.Write(code);
                }

                return 0;
            });
            commandLineApplication.Execute(args);
        }
    }
}