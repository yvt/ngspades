using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
    abstract class ValueMarshaller
    {
        static readonly Dictionary<Type, ValueMarshaller> marshallers = new Dictionary<Type, ValueMarshaller> {
            {typeof(byte), new SimpleValueMarshaller(typeof(byte))},
            {typeof(short), new SimpleValueMarshaller(typeof(short))},
            {typeof(int), new SimpleValueMarshaller(typeof(int))},
            {typeof(long), new SimpleValueMarshaller(typeof(long))},
            {typeof(sbyte), new SimpleValueMarshaller(typeof(sbyte))},
            {typeof(ushort), new SimpleValueMarshaller(typeof(ushort))},
            {typeof(uint), new SimpleValueMarshaller(typeof(uint))},
            {typeof(ulong), new SimpleValueMarshaller(typeof(ulong))},
            {typeof(float), new SimpleValueMarshaller(typeof(float))},
            {typeof(double), new SimpleValueMarshaller(typeof(double))},
            {typeof(string), new StringValueMarshaller()},
        };

        public static ValueMarshaller GetMarshaller(Type type)
        {
            ValueMarshaller marshaller = null;

            lock (marshallers)
            {
                if (marshallers.ContainsKey(type))
                {
                    return marshallers[type];
                }

                var typeInfo = type.GetTypeInfo();

                if (typeInfo.IsSubclassOf(typeof(ValueType)))
                {
                    marshaller = new SimpleValueMarshaller(type);
                }

                if (typeInfo.IsInterface &&
                    typeof(IUnknown).GetTypeInfo().IsAssignableFrom(typeInfo))
                {
                    marshaller = new InterfaceValueMarshaller(type);
                }

                if (marshaller != null)
                {
                    marshallers.Add(type, marshaller);
                }
                else
                {
                    throw new InvalidOperationException($"Don't know how to marshal {type.FullName}");
                }
            }

            return marshaller;
        }

        public abstract ValueToNativeMarshallerGenerator CreateToNativeGenerator(ILGenerator generator);
        public abstract ValueToRuntimeMarshallerGenerator CreateToRuntimeGenerator(ILGenerator generator);

        public abstract Type NativeParameterType { get; }
    }

    abstract class ValueToNativeMarshallerGenerator
    {
        /**
         * Emits a code that converts a managed value into a native one.
         */
        public abstract void EmitToNative(Storage inputStorage, Storage outputStorage);
        public abstract void EmitDestructNativeValue(Storage nativeStorage);
    }

    abstract class ValueToRuntimeMarshallerGenerator
    {
        /**
         * Emits a code that converts a native value into a managed one.
         * The original native value can possibly be destroyed if `move` is true.
         */
        public abstract void EmitToRuntime(Storage inputStorage, Storage outputStorage, bool move);
        public abstract void EmitDestructNativeValue(Storage nativeStorage);
    }
}