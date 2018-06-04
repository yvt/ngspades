//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Numerics;
using Xunit;

namespace Ngs.Engine.Tests {
    public class VectorTest {
        [Fact]
        public void Vector2Extend() {
            Assert.Equal(new Vector3(1, 2, 3), new Vector2(1, 2).Extend(3));
        }

        [Fact]
        public void Vector3Extend() {
            Assert.Equal(new Vector4(1, 2, 3, 4), new Vector3(1, 2, 3).Extend(4));
        }

        [Fact]
        public void Vector4Truncate() {
            Assert.Equal(new Vector3(1, 2, 3), new Vector4(1, 2, 3, 4).Truncate());
        }

        [Fact]
        public void Vector3Truncate() {
            Assert.Equal(new Vector2(1, 2), new Vector3(1, 2, 3).Truncate());
        }

        [Theory]
        [InlineData(10f, 0)]
        [InlineData(20f, 1)]
        public void Vector2GetElement(float value, int index) {
            Assert.Equal(value, new Vector2(10, 20).GetElementAt(index));
        }

        [Theory]
        [InlineData(int.MinValue)]
        [InlineData(-1)]
        [InlineData(2)]
        [InlineData(int.MaxValue)]
        public void Vector2GetElementFail(int index) {
            Assert.Throws<IndexOutOfRangeException>(() => {
                new Vector2(10, 20).GetElementAt(index);
            });
        }

        [Fact]
        public void Vector2Element() {
            var v = new Vector2(10, 20);
            v.ElementAt(0) = 30;
            Assert.Equal(new Vector2(30, 20), v);

            v.ElementAt(1) = 40;
            Assert.Equal(new Vector2(30, 40), v);
        }

        [Theory]
        [InlineData(int.MinValue)]
        [InlineData(-1)]
        [InlineData(2)]
        [InlineData(int.MaxValue)]
        public void Vector2ElementFail(int index) {
            Assert.Throws<IndexOutOfRangeException>(() => {
                var v = new Vector2(10, 20);
                v.ElementAt(index);
            });
        }

        [Theory]
        [InlineData(10f, 0)]
        [InlineData(20f, 1)]
        [InlineData(30f, 2)]
        public void Vector3GetElement(float value, int index) {
            Assert.Equal(value, new Vector3(10, 20, 30).GetElementAt(index));
        }

        [Theory]
        [InlineData(int.MinValue)]
        [InlineData(-1)]
        [InlineData(3)]
        [InlineData(int.MaxValue)]
        public void Vector3GetElementFail(int index) {
            Assert.Throws<IndexOutOfRangeException>(() => {
                new Vector3(10, 20, 30).GetElementAt(index);
            });
        }

        [Fact]
        public void Vector3Element() {
            var v = new Vector3(10, 20, 30);
            v.ElementAt(0) = 55;
            Assert.Equal(new Vector3(55, 20, 30), v);

            v.ElementAt(1) = 66;
            Assert.Equal(new Vector3(55, 66, 30), v);

            v.ElementAt(2) = 77;
            Assert.Equal(new Vector3(55, 66, 77), v);
        }

        [Theory]
        [InlineData(int.MinValue)]
        [InlineData(-1)]
        [InlineData(3)]
        [InlineData(int.MaxValue)]
        public void Vector3ElementFail(int index) {
            Assert.Throws<IndexOutOfRangeException>(() => {
                var v = new Vector3(10, 20, 30);
                v.ElementAt(index);
            });
        }

        [Theory]
        [InlineData(10f, 0)]
        [InlineData(20f, 1)]
        [InlineData(30f, 2)]
        [InlineData(40f, 3)]
        public void Vector4GetElement(float value, int index) {
            Assert.Equal(value, new Vector4(10, 20, 30, 40).GetElementAt(index));
        }

        [Theory]
        [InlineData(int.MinValue)]
        [InlineData(-1)]
        [InlineData(4)]
        [InlineData(int.MaxValue)]
        public void Vector4GetElementFail(int index) {
            Assert.Throws<IndexOutOfRangeException>(() => {
                new Vector4(10, 20, 30, 40).GetElementAt(index);
            });
        }

        [Fact]
        public void Vector4Element() {
            var v = new Vector4(10, 20, 30, 40);
            v.ElementAt(0) = 55;
            Assert.Equal(new Vector4(55, 20, 30, 40), v);

            v.ElementAt(1) = 66;
            Assert.Equal(new Vector4(55, 66, 30, 40), v);

            v.ElementAt(2) = 77;
            Assert.Equal(new Vector4(55, 66, 77, 40), v);

            v.ElementAt(3) = 88;
            Assert.Equal(new Vector4(55, 66, 77, 88), v);
        }

        [Theory]
        [InlineData(int.MinValue)]
        [InlineData(-1)]
        [InlineData(4)]
        [InlineData(int.MaxValue)]
        public void Vector4ElementFail(int index) {
            Assert.Throws<IndexOutOfRangeException>(() => {
                var v = new Vector4(10, 20, 30, 40);
                v.ElementAt(index);
            });
        }
    }
}
