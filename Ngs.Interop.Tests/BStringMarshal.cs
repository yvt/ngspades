using System;
using Xunit;
namespace Ngs.Interop.Tests {
    public class BString_ShouldHave : IDisposable {
        IntPtr p = NgscomMarshal.AllocBString ("Pudding in the freezy🎈🎉!");

        public void Dispose () {
            NgscomMarshal.FreeBString (p);
            p = IntPtr.Zero;
        }

        [Fact]
        public void NonNullPointer () {
            Assert.NotEqual (IntPtr.Zero, p);
        }

        [Fact]
        public void IntendedContents () {
            Assert.Equal ("Pudding in the freezy🎈🎉!", NgscomMarshal.BStringToString (p));
        }

        [Fact]
        public void IntendedLength () {
            Assert.Equal (30, NgscomMarshal.GetBStringLength (p));
        }
    }
}