//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Reflection;
using System.Reflection.Emit;

namespace Ngs.Interop.Marshaller
{
    sealed class InterfaceValueMarshaller : ValueMarshaller
    {
        static void EmitDestruct(ILGenerator generator, Storage nativeStorage)
        {
            var ptrIsNullLabel = generator.DefineLabel();

            // is the interface pointer null?
            nativeStorage.EmitLoad();
            generator.Emit(OpCodes.Ldc_I4_0);
            generator.Emit(OpCodes.Conv_I);
            generator.Emit(OpCodes.Beq, ptrIsNullLabel);

            // get the interface pointer for arg #1
            nativeStorage.EmitLoad();

            // get the interface pointer for vtable entry fetch
            nativeStorage.EmitLoad();

            // get vtable
            generator.Emit(OpCodes.Ldind_I);

            // load the vtable element "IUnknown::Release"
            int vtableIndex = 2;
            generator.Emit(OpCodes.Sizeof, typeof(IntPtr));
            generator.Emit(OpCodes.Ldc_I4, vtableIndex);
            generator.Emit(OpCodes.Mul);
            generator.Emit(OpCodes.Conv_I);
            generator.Emit(OpCodes.Add);
            generator.Emit(OpCodes.Ldind_I);

            generator.EmitCalli(OpCodes.Calli, CallingConventions.Standard, typeof(uint),
                new[] { typeof(IntPtr) }, null);

            generator.Emit(OpCodes.Pop);

            generator.MarkLabel(ptrIsNullLabel);
        }

        private sealed class ToNativeGenerator : ValueToNativeMarshallerGenerator
        {
            InterfaceValueMarshaller parent;
            ILGenerator generator;

            public ToNativeGenerator(ILGenerator generator, InterfaceValueMarshaller parent)
            {
                this.generator = generator;
                this.parent = parent;
            }

            static MethodInfo getCcwForInterfaceMethodGeneric = typeof(NgscomMarshal)
                .GetTypeInfo().GetDeclaredMethod(nameof(NgscomMarshal.GetCcwForInterface));

            public override void EmitToNative(Storage inputStorage, Storage outputStorage)
            {
                var getCcwForInterfacePtrMethod = getCcwForInterfaceMethodGeneric.MakeGenericMethod(parent.type);

                inputStorage.EmitLoad();

                generator.EmitCall(OpCodes.Call, getCcwForInterfacePtrMethod, null);

                outputStorage.EmitStore();
            }

            public override void EmitDestructNativeValue(Storage nativeStorage)
            {
                EmitDestruct(generator, nativeStorage);
            }
        }

        private sealed class ToRuntimeGenerator : ValueToRuntimeMarshallerGenerator
        {
            InterfaceValueMarshaller parent;
            ILGenerator generator;

            public ToRuntimeGenerator(ILGenerator generator, InterfaceValueMarshaller parent)
            {
                this.generator = generator;
                this.parent = parent;
            }

            static MethodInfo getRcwForInterfacePtrMethodGeneric = typeof(NgscomMarshal)
                .GetTypeInfo().GetDeclaredMethod(nameof(NgscomMarshal.GetRcwForInterfacePtr));

            public override void EmitToRuntime(Storage inputStorage, Storage outputStorage, bool move)
            {
                var getRcwForInterfacePtrMethod = getRcwForInterfacePtrMethodGeneric.MakeGenericMethod(parent.type);

                inputStorage.EmitLoad();
                if (move)
                {
                    generator.Emit(OpCodes.Ldc_I4_0); // addRef = false; interface pointer's ownership is transferred to RCW
                }
                else
                {
                    generator.Emit(OpCodes.Ldc_I4_1); // addRef = true; interface pointer's cloned
                }

                generator.EmitCall(OpCodes.Call, getRcwForInterfacePtrMethod, null);

                outputStorage.EmitStore();
            }

            public override void EmitDestructNativeValue(Storage nativeStorage)
            {
                EmitDestruct(generator, nativeStorage);
            }
        }

        Type type;

        public InterfaceValueMarshaller(Type type)
        {
            if (!type.GetTypeInfo().IsInterface)
            {
                throw new InvalidOperationException();
            }
            this.type = type;
        }

        public override ValueToNativeMarshallerGenerator CreateToNativeGenerator(ILGenerator generator)
        {
            return new ToNativeGenerator(generator, this);
        }

        public override ValueToRuntimeMarshallerGenerator CreateToRuntimeGenerator(ILGenerator generator)
        {
            return new ToRuntimeGenerator(generator, this);
        }

        public override Type NativeParameterType
        {
            get { return typeof(IntPtr); }
        }
    }
}