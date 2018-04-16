//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Reflection.Emit;

namespace Ngs.Interop.Marshaller {
    // FIXME: unused?
    sealed class SimplePointerValueMarshaller : ValueMarshaller {
        private sealed class ToNativeGenerator : ValueToNativeMarshallerGenerator {
            ILGenerator generator;

            public ToNativeGenerator (ILGenerator generator) {
                this.generator = generator;
            }

            public override void EmitToNative (Storage inputStorage, Storage outputStorage) {
                inputStorage.EmitLoad ();
                outputStorage.EmitStore ();
            }

            public override void EmitDestructNativeValue (Storage nativeStorage) { }
        }

        private sealed class ToRuntimeGenerator : ValueToRuntimeMarshallerGenerator {
            ILGenerator generator;

            public ToRuntimeGenerator (ILGenerator generator) {
                this.generator = generator;
            }

            public override void EmitToRuntime (Storage inputStorage, Storage outputStorage, bool move) {
                inputStorage.EmitLoad ();
                outputStorage.EmitStore ();
            }

            public override void EmitDestructNativeValue (Storage nativeStorage) { }
        }

        Type type;

        public SimplePointerValueMarshaller (Type type) {
            this.type = type;
        }

        public override ValueToNativeMarshallerGenerator CreateToNativeGenerator (ILGenerator generator) {
            return new ToNativeGenerator (generator);
        }

        public override ValueToRuntimeMarshallerGenerator CreateToRuntimeGenerator (ILGenerator generator) {
            return new ToRuntimeGenerator (generator);
        }

        public override Type NativeParameterType {
            get { return typeof (IntPtr); }
        }
    }
}