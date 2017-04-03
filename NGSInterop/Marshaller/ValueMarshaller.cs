using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
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
        };

        public static ValueMarshaller GetMarshaller(Type type)
        {
            ValueMarshaller marshaller = null;

            lock (marshallers) {
                if (marshallers.ContainsKey(type)) {
                    return marshallers[type];
                }

				var typeInfo = type.GetTypeInfo();

				if (typeInfo.IsSubclassOf(typeof(Enum))) {
                    marshaller = new SimpleValueMarshaller(type);
                }

				if (typeInfo.IsInterface && typeInfo.IsSubclassOf(typeof(IUnknown)))
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
        public abstract void EmitToNative(Storage inputStorage, Storage outputStorage);
    }

    abstract class ValueToRuntimeMarshallerGenerator
    {
        public abstract void EmitToRuntime(Storage inputStorage, Storage outputStorage);
    }
}