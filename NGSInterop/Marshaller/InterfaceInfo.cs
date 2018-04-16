//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;

namespace Ngs.Interop.Marshaller {
    sealed class InterfaceInfo {
        public Type Type { get; }
        public TypeInfo TypeInfo { get; }
        public Guid ComGuid { get; }

        public InterfaceInfo (Type type) {
            if (type == null) {
                throw new ArgumentNullException (nameof (type));
            }

            Type = type;
            TypeInfo = type.GetTypeInfo ();

            if (!TypeInfo.IsInterface) {
                throw new ArgumentException ($"The specified type {type.FullName} is not an interface type.", nameof (type));
            }
            if (!typeof (IUnknown).GetTypeInfo ().IsAssignableFrom (TypeInfo)) {
                throw new ArgumentException ($"The specified type {type.FullName} is not marshallable.", nameof (type));
            }

            var guidAttr = type.GetTypeInfo ().GetCustomAttribute<System.Runtime.InteropServices.GuidAttribute> ();
            if (guidAttr != null) {
                ComGuid = new Guid (guidAttr.Value);
            }
        }

        IEnumerable<Type> allImplementedInterfaces;

        private static void BuildAllImplementedInterfaces (HashSet<Type> types, Type type) {
            if (types.Add (type)) {
                foreach (var t in type.GetTypeInfo ().ImplementedInterfaces) {
                    BuildAllImplementedInterfaces (types, t);
                }
            }
        }

        public IEnumerable<Type> AllImplementedInterfaces {
            get {
                var ret = allImplementedInterfaces;

                if (ret == null) {
                    var typeSet = new HashSet<Type> ();
                    BuildAllImplementedInterfaces (typeSet, Type);
                    ret = allImplementedInterfaces = typeSet.ToArray ();
                }

                return ret;
            }
        }

        IEnumerable<ComMethodInfo> comMethodInfos;

        private static void BuildComMethodInfos (HashSet<Type> processedTypeSet, Type type, ref int vtableOffset, List<ComMethodInfo> outMethods) {
            var typeInfo = type.GetTypeInfo ();

            foreach (var t in typeInfo.ImplementedInterfaces) {
                BuildComMethodInfos (processedTypeSet, t, ref vtableOffset, outMethods);
            }

            if (processedTypeSet.Contains (type)) {
                return;
            }

            foreach (var method in type.GetTypeInfo ().DeclaredMethods) {
                outMethods.Add (new ComMethodInfo (method, vtableOffset++, ComMethodType.RcwMethod));
            }
        }

        /**
         * Includes inherited methods.
         */
        public IEnumerable<ComMethodInfo> ComMethodInfos {
            get {
                var ret = comMethodInfos;

                if (ret == null) {
                    var processedTypeSet = new HashSet<Type> ();
                    int vtableOffset = 0;
                    var methods = new List<ComMethodInfo> ();
                    BuildComMethodInfos (new HashSet<Type> (), Type, ref vtableOffset, methods);
                    comMethodInfos = ret = methods.ToArray ();
                }

                return ret;
            }
        }

    }

    enum ComMethodType {
        RcwMethod,
        FunctionPtrThunk
    }

    sealed class ComMethodInfo {
        public MethodInfo MethodInfo { get; }
        public int VTableOffset { get; }
        public bool PreserveSig { get; }
        public ComMethodType MethodType { get; }

        public ComMethodInfo (MethodInfo baseMethod, int vtableOffset, ComMethodType methodType) {
            MethodInfo = baseMethod;
            VTableOffset = vtableOffset;
            MethodType = methodType;

            if (methodType == ComMethodType.FunctionPtrThunk ||
                baseMethod.GetCustomAttribute<System.Runtime.InteropServices.PreserveSigAttribute> () != null) {
                PreserveSig = true;
            }
        }

        public bool IsFirstParameterFunctionPtr => MethodType == ComMethodType.FunctionPtrThunk;

        public bool IsFirstNativeParameterInterfacePtr => MethodType == ComMethodType.RcwMethod;

        public bool HasReturnValueParameter => !PreserveSig && MethodInfo.ReturnType != typeof (void);

        public bool ReturnsHresult => !PreserveSig;

        public bool ReturnsReturnValue => PreserveSig && MethodInfo.ReturnType != typeof (void);

        private IEnumerable<ComMethodParameterInfo> parameterInfos;

        /**
         * Doesn't include "this" pointer.
         */
        public IEnumerable<ComMethodParameterInfo> ParameterInfos {
            get {
                var ret = parameterInfos;
                if (ret == null) {
                    var list = MethodInfo.GetParameters ().Select ((p) => new ComMethodParameterInfo (this, p)).ToList ();

                    if (IsFirstParameterFunctionPtr) {
                        list.RemoveAt (0);
                    }

                    if (HasReturnValueParameter) {
                        list.Add (new ComMethodParameterInfo (this, null));
                    }

                    ret = parameterInfos = list.ToArray ();
                }
                return ret;
            }
        }

        ValueMarshaller returnValueMarshaller;

        public ValueMarshaller ReturnValueMarshaller {
            get {
                var ret = returnValueMarshaller;
                if (ret == null && ReturnsReturnValue) {
                    ret = returnValueMarshaller = ValueMarshaller.GetMarshaller (MethodInfo.ReturnType);
                }
                return ret;
            }
        }

        public Type NativeReturnType => ReturnsHresult ? typeof (int) :
            ReturnValueMarshaller?.NativeParameterType ?? typeof (void);
    }

    sealed class ComMethodParameterInfo {
        public ParameterInfo ParameterInfo { get; }
        public bool IsReturnValue => ParameterInfo == null;
        public ComMethodInfo ComMethodInfo { get; }
        public bool IsByRef => IsReturnValue ? true : ParameterInfo.ParameterType.IsByRef;
        public bool IsMissingInOut => !(ParameterInfo.IsIn || ParameterInfo.IsOut) || (IsByRef && !ParameterInfo.IsOut);
        public bool IsOut => IsReturnValue ? true : IsMissingInOut ? IsByRef : ParameterInfo.IsOut;
        public bool IsIn => IsReturnValue ? false : IsMissingInOut ? true : ParameterInfo.IsIn;

        public ComMethodParameterInfo (ComMethodInfo methodInfo, ParameterInfo paramInfo) {
            this.ComMethodInfo = methodInfo;
            this.ParameterInfo = paramInfo;
        }

        ValueMarshaller valueMarshaller;
        public ValueMarshaller ValueMarshaller {
            get {
                var ret = valueMarshaller;

                if (ret == null) {
                    ret = valueMarshaller = ValueMarshaller.GetMarshaller (Type);
                }

                return ret;
            }
        }

        /** For by-ref parameters, this returns a non-by-ref type. */
        public Type Type => IsReturnValue ? ComMethodInfo.MethodInfo.ReturnType :
            IsByRef ? ParameterInfo.ParameterType.GetElementType () :
            ParameterInfo.ParameterType;

        Type NativeTypeWithoutRef => ValueMarshaller.NativeParameterType;

        // FIXME: this is, in a exact sense, not a native type (oh what have I done...)
        public Type NativeType => (IsReturnValue || ParameterInfo.IsOut) ? Type.MakePointerType () : Type;
    }
}