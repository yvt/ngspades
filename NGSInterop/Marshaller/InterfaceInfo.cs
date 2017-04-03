using System;
using System.Collections;
using System.Collections.Generic;
using System.Reflection;
using System.Linq;

namespace Ngs.Interop.Marshaller
{
	sealed class InterfaceInfo
	{
		private class MethodListImpl : IEnumerable<ComMethodInfo>
		{
			private Type type;

			public MethodListImpl(Type type)
			{
				this.type = type;
			}

			public IEnumerator<ComMethodInfo> GetEnumerator()
			{
				var stack = new Stack<Tuple<Type, bool>>();
				stack.Push(Tuple.Create(type, false));

				var processedTypeSet = new HashSet<Type>();

				int vtableOffset = 0;

				while (stack.Count > 0)
				{
					var e = stack.Pop();
					if (processedTypeSet.Contains(e.Item1))
					{
						continue;
					}
					if (!e.Item2)
					{
						stack.Push(Tuple.Create(type, true));
						foreach (var t in type.GetTypeInfo().ImplementedInterfaces)
						{
							stack.Push(Tuple.Create(t, false));
						}
					}
					else
					{
						processedTypeSet.Add(e.Item1);

						var t = e.Item1;

						foreach (var method in t.GetTypeInfo().DeclaredMethods)
						{
							yield return new ComMethodInfo(method, vtableOffset++);
						}
					}
				}
			}

			IEnumerator IEnumerable.GetEnumerator()
			{
				foreach (var e in this)
				{
					yield return e;
				}
			}
		}

		public Type Type { get; }
		public TypeInfo TypeInfo { get; }
		public Guid ComGuid { get; }

		public InterfaceInfo(Type type)
		{
			if (type == null)
			{
				throw new ArgumentNullException(nameof(type));
			}
			
			Type = type;
			TypeInfo = type.GetTypeInfo();

			if (!TypeInfo.IsInterface)
			{
				throw new ArgumentException("The specified type is not an interface type.", nameof(type));
			}
			if (!TypeInfo.IsSubclassOf(typeof(IUnknown)))
			{
				throw new ArgumentException("The specified type is not marshallable.", nameof(type));
			}

			var guidAttr = type.GetTypeInfo().GetCustomAttribute<System.Runtime.InteropServices.GuidAttribute>();
			if (guidAttr != null)
			{
				ComGuid = new Guid(guidAttr.Value);
			}
		}

		IEnumerable<Type> allImplementedInterfaces;

		private static void BuildAllImplementedInterfaces(HashSet<Type> types, Type type)
		{
			if (types.Add(type))
			{
				foreach (var t in type.GetTypeInfo().ImplementedInterfaces)
				{
					BuildAllImplementedInterfaces(types, t);
				}
			}
		}

		public IEnumerable<Type> AllImplementedInterfaces
		{
			get
			{
				var ret = allImplementedInterfaces;

				if (ret == null)
				{
					var typeSet = new HashSet<Type>();
					BuildAllImplementedInterfaces(typeSet, Type);
					ret = allImplementedInterfaces = typeSet.ToArray();
				}

				return ret;
			}
		}

		IEnumerable<ComMethodInfo> comMethodInfos;

		private static void BuildComMethodInfos(HashSet<Type> processedTypeSet, Type type, ref int vtableOffset, List<ComMethodInfo> outMethods)
		{
			var typeInfo = type.GetTypeInfo();

			foreach (var t in typeInfo.ImplementedInterfaces)
			{
				BuildComMethodInfos(processedTypeSet, t, ref vtableOffset, outMethods);
			}

			if (processedTypeSet.Contains(type))
			{
				return;
			}

			foreach (var method in type.GetTypeInfo().DeclaredMethods)
			{
				outMethods.Add(new ComMethodInfo(method, vtableOffset++));
			}
		}

		public IEnumerable<ComMethodInfo> ComMethodInfos
		{
			get
			{
				var ret = comMethodInfos;

				if (ret == null)
				{
					var processedTypeSet = new HashSet<Type>();
					int vtableOffset = 0;
					var methods = new List<ComMethodInfo>();
					BuildComMethodInfos(new HashSet<Type>(), Type, ref vtableOffset, methods);
					comMethodInfos = ret = methods.ToArray();
				}

				return ret;
			}
		}

	}

	sealed class ComMethodInfo
	{
		public MethodInfo MethodInfo { get; }
		public int VTableOffset { get; }
		public bool PreserveSig { get; }

		public ComMethodInfo(MethodInfo baseMethod, int vtableOffset)
		{
			MethodInfo = baseMethod;
			VTableOffset = vtableOffset;

			if (baseMethod.GetCustomAttribute<System.Runtime.InteropServices.PreserveSigAttribute>() != null)
			{
				PreserveSig = true;
			}
		}

		public bool HasReturnValueParameter => !PreserveSig && MethodInfo.ReturnType != typeof(void);

		public bool ReturnsHresult => PreserveSig;

		public bool ReturnsReturnValue => PreserveSig && MethodInfo.ReturnType != typeof(void);

		private IEnumerable<ComMethodParameterInfo> parameterInfos;

		public IEnumerable<ComMethodParameterInfo> ParameterInfos
		{
			get
			{
				var ret = parameterInfos;
				if (ret == null)
				{
					var list = MethodInfo.GetParameters().Select((p) => new ComMethodParameterInfo(this, p)).ToList();

					if (HasReturnValueParameter)
					{
						list.Add(new ComMethodParameterInfo(this, null));
					}

					ret = parameterInfos = list.ToArray();
				}
				return ret;
			}
		}

		ValueMarshaller returnValueMarshaller;

		public ValueMarshaller ReturnValueMarshaller
		{
			get
			{
				var ret = returnValueMarshaller;
				if (ret == null && ReturnsReturnValue)
				{
					ret = returnValueMarshaller = ValueMarshaller.GetMarshaller(MethodInfo.ReturnType);
				}
				return ret;
			}
		}

		public Type NativeReturnType => ReturnsHresult ? typeof(int) :
			returnValueMarshaller?.NativeParameterType ?? typeof(void);
	}

	sealed class ComMethodParameterInfo
	{
		public ParameterInfo ParameterInfo { get; }
		public bool IsReturnValue => ParameterInfo == null;
		public ComMethodInfo ComMethodInfo { get; }
		public bool IsOut => IsReturnValue ? true : ParameterInfo.IsOut;
		public bool IsIn => IsReturnValue ? false : ParameterInfo.IsIn;

		public ComMethodParameterInfo(ComMethodInfo methodInfo, ParameterInfo paramInfo)
		{
			this.ComMethodInfo = methodInfo;
			this.ParameterInfo = paramInfo;
		}

		ValueMarshaller valueMarshaller;
		public ValueMarshaller ValueMarshaller
		{
			get
			{
				var ret = valueMarshaller;

				if (ret == null)
				{
					ret = valueMarshaller = ValueMarshaller.GetMarshaller(Type);
				}

				return ret;
			}
		}

		public Type Type => IsReturnValue ? ComMethodInfo.MethodInfo.ReturnType :
														 ParameterInfo.ParameterType;

		public Type NativeType => (IsReturnValue || ParameterInfo.IsOut) ? Type.MakePointerType() : Type;
	}
}
