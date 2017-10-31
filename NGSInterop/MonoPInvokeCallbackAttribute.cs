//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
namespace Ngs.Interop
{
    /// <summary>
	/// Enables creation of functions pointers from the annotated functions.
    /// </summary>
    /// <remarks>
	/// <para>
	/// This attribute forces a native-to-managed wrapper to be generated for the function it is
	/// applied on.
	/// (This is required to use
	/// <see cref="System.Runtime.InteropServices.Marshal.GetFunctionPointerForDelegate" /> in AOT
	/// environments such as Mono for iOS since JIT compilation is disabled in such environments.)
	/// </para>
    /// <para>
    /// A class with the same name is defined by the Mono runtime, but since it is specific to Mono,
	/// an attempt to use it results in a compilation error or a type load error (in runtime).
	/// We can accomplish the same effect in a cross platform way by defining a class with the same
	/// name in our assembly because the Mono runtime only checks the class name.
    /// This technique has been suggested in
	/// <see href="http://answers.unity3d.com/questions/191234/unity-ios-function-pointers.html">a post</see>
	/// at the Unity 3D forum.
	/// </para>
    /// </remarks>
    [AttributeUsage(AttributeTargets.Method)]
    public sealed class MonoPInvokeCallbackAttribute : Attribute
    {
        private Type type;

        /// <summary>
        /// Initializes a new instance of the <see cref="MonoPInvokeCallbackAttribute" /> class.
        /// </summary>
        /// <param name="t">Delegate type.</param>
        public MonoPInvokeCallbackAttribute(Type t)
        {
            type = t;
        }
    }
}
