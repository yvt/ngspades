﻿using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Linq;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Ngs.Interop.Marshaller
{
	sealed class StringValueMarshaller : ValueMarshaller
	{
		static readonly MethodInfo allocBStringMethod = typeof(NgscomMarshal)
			.GetRuntimeMethod(nameof(NgscomMarshal.AllocBString), new [] {typeof(string)});

		static readonly MethodInfo bStringToStringMethod = typeof(NgscomMarshal)
			.GetRuntimeMethod(nameof(NgscomMarshal.BStringToString), new [] {typeof(IntPtr)});

		static readonly MethodInfo freeBStringMethod = typeof(NgscomMarshal)
			.GetRuntimeMethod(nameof(NgscomMarshal.FreeBString), new Type[] {typeof(IntPtr)});
			
		private sealed class ToNativeGenerator : ValueToNativeMarshallerGenerator
		{
			ILGenerator generator;

			public ToNativeGenerator(ILGenerator generator)
			{
				this.generator = generator;
			}


			public override void EmitToNative(Storage inputStorage, Storage outputStorage)
			{
				inputStorage.EmitLoad();
				generator.Emit(OpCodes.Call, allocBStringMethod);
				outputStorage.EmitStore();
			}

        	public override void EmitDestructNativeValue(Storage nativeStorage)
			{
				nativeStorage.EmitLoad();
				generator.Emit(OpCodes.Call, freeBStringMethod);
			}
		}

		private sealed class ToRuntimeGenerator : ValueToRuntimeMarshallerGenerator
		{
			ILGenerator generator;

			public ToRuntimeGenerator(ILGenerator generator)
			{
				this.generator = generator;
			}

			public override void EmitToRuntime(Storage inputStorage, Storage outputStorage)
			{
				inputStorage.EmitLoad();
				generator.Emit(OpCodes.Call, bStringToStringMethod);
				outputStorage.EmitStore();

				inputStorage.EmitLoad();
				generator.Emit(OpCodes.Call, freeBStringMethod);
			}

        	public override void EmitDestructNativeValue(Storage nativeStorage)
			{
				nativeStorage.EmitLoad();
				generator.Emit(OpCodes.Call, freeBStringMethod);
			}
		}

		public override ValueToNativeMarshallerGenerator CreateToNativeGenerator(ILGenerator generator)
		{
			return new ToNativeGenerator(generator);
		}

		public override ValueToRuntimeMarshallerGenerator CreateToRuntimeGenerator(ILGenerator generator)
		{
			return new ToRuntimeGenerator(generator);
		}

		public override Type NativeParameterType
		{
			get { return typeof(void*); }
		}
	}
}