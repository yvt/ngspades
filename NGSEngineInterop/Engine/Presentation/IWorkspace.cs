using System;
using System.Runtime.InteropServices;
using Ngs.Interop;

namespace Ngs.Engine.Presentation
{
	[Guid("605d9976-ab88-47cf-b68b-e1c2dfeaaa99")]
	public interface IWorkspace : IUnknown
	{
        IPresentationContext Context { get; }
        
		void CommitFrame();
	}
}
