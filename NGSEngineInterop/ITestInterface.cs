using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine {
	[Guid("35edff15-0b38-47d8-9b7c-e00fa2acdf9d")]
	public interface ITestInterface : IUnknown
	{
		string HogeAttr { get; set; }
		string Hello(string str);
		void SimpleMethod();
	}

}