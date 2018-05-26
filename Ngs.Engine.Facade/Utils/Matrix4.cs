//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Utils {
    /// <summary>
    /// Represents a 4x4 matrix.
    /// </summary>
    /// <remarks>
    /// The components of matrices are arranged in a column-major order in NgsPF,
    /// as opposed to <see cref="System.Numerics.Matrix4x4" />.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public struct Matrix4 {
        /// <summary>
        /// The first column.
        /// </summary>
        public Vector4 C1;

        /// <summary>
        /// The second column.
        /// </summary>
        public Vector4 C2;

        /// <summary>
        /// The third column.
        /// </summary>
        public Vector4 C3;

        /// <summary>
        /// The fourth column.
        /// </summary>
        public Vector4 C4;

        /// <summary>
        /// Creates a 4x4 matrix from the specified components.
        /// </summary>
        /// <param name="m11">The first element in the first row.</param>
        /// <param name="m12">The second element in the first row.</param>
        /// <param name="m13">The third element in the first row.</param>
        /// <param name="m14">The fourth element in the first row.</param>
        /// <param name="m21">The first element in the second row.</param>
        /// <param name="m22">The second element in the second row.</param>
        /// <param name="m23">The third element in the second row.</param>
        /// <param name="m24">The fourth element in the second row.</param>
        /// <param name="m31">The first element in the third row.</param>
        /// <param name="m32">The second element in the third row.</param>
        /// <param name="m33">The third element in the third row.</param>
        /// <param name="m34">The fourth element in the third row.</param>
        /// <param name="m41">The first element in the fourth row.</param>
        /// <param name="m42">The second element in the fourth row.</param>
        /// <param name="m43">The third element in the fourth row.</param>
        /// <param name="m44">The fourth element in the fourth row.</param>
        /// <remarks>
        /// The components are specified in a row-major order in the parameters
        /// of this constructor.
        /// </remarks>
        public Matrix4(
            float m11, float m12, float m13, float m14,
            float m21, float m22, float m23, float m24,
            float m31, float m32, float m33, float m34,
            float m41, float m42, float m43, float m44
        ) {
            C1 = new Vector4(m11, m21, m31, m41);
            C2 = new Vector4(m12, m22, m32, m42);
            C3 = new Vector4(m13, m23, m33, m43);
            C4 = new Vector4(m14, m24, m34, m44);
        }

        /// <summary>
        /// Adds two matricies.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns>The addition result.</returns>
        public static Matrix4 operator +(Matrix4 m1, Matrix4 m2) {
            return new Matrix4()
            {
                C1 = m1.C1 + m2.C1,
                C2 = m1.C2 + m2.C2,
                C3 = m1.C3 + m2.C3,
                C4 = m1.C4 + m2.C4,
            };
        }

        /// <summary>
        /// Subtracts the second matrix from the first one.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns>The subtraction result.</returns>
        public static Matrix4 operator -(Matrix4 m1, Matrix4 m2) {
            return new Matrix4()
            {
                C1 = m1.C1 - m2.C1,
                C2 = m1.C2 - m2.C2,
                C3 = m1.C3 - m2.C3,
                C4 = m1.C4 - m2.C4,
            };
        }

        /// <summary>
        /// Multiplies two matricies.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns>The multiplication result.</returns>
        public static Matrix4 operator *(Matrix4 m1, Matrix4 m2) {
            var mt = Matrix4.Transpose(m1);
            // TODO: optimize this to (maybe) eliminate the use of dot products?

            return new Matrix4()
            {
                C1 = new Vector4(
                        Vector4.Dot(mt.C1, m2.C1),
                        Vector4.Dot(mt.C2, m2.C1),
                        Vector4.Dot(mt.C3, m2.C1),
                        Vector4.Dot(mt.C4, m2.C1)
                    ),
                C2 = new Vector4(
                        Vector4.Dot(mt.C1, m2.C2),
                        Vector4.Dot(mt.C2, m2.C2),
                        Vector4.Dot(mt.C3, m2.C2),
                        Vector4.Dot(mt.C4, m2.C2)
                    ),
                C3 = new Vector4(
                        Vector4.Dot(mt.C1, m2.C3),
                        Vector4.Dot(mt.C2, m2.C3),
                        Vector4.Dot(mt.C3, m2.C3),
                        Vector4.Dot(mt.C4, m2.C3)
                    ),
                C4 = new Vector4(
                        Vector4.Dot(mt.C1, m2.C4),
                        Vector4.Dot(mt.C2, m2.C4),
                        Vector4.Dot(mt.C3, m2.C4),
                        Vector4.Dot(mt.C4, m2.C4)
                    ),
            };
        }

        /// <summary>
        /// Multiplies a matrix and vector.
        /// </summary>
        /// <param name="m">The matrix used as the left operand.</param>
        /// <param name="v">The vector used as the right operand.</param>
        /// <returns>The multiplication result.</returns>
        public static Vector4 operator *(Matrix4 m, Vector4 v) =>
            m.C1 * v.X + m.C2 * v.Y + m.C3 * v.Z + m.C4 * v.W;

        /// <summary>
        /// Transforms a three-dimensional point represented by a specified vector by this matrix.
        /// </summary>
        /// <param name="v">The vector to transform.</param>
        /// <returns>The transformed vector.</returns>
        public Vector3 TransformPoint(Vector3 v) {
            var factor = this * v.Extend(1);
            factor *= 1 / factor.W;
            return factor.Truncate();
        }

        /// <summary>
        /// Transforms a three-dimensional (directional) vector by this matrix.
        /// </summary>
        /// <param name="v">The vector to transform.</param>
        /// <returns>The transformed vector.</returns>
        public Vector3 TransformVector(Vector3 v) => (this * v.Extend(0)).Truncate();

        /// <summary>
        /// Returns a flag indicating whether two specified matrices are equal.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns><c>true</c> if the matrices are equal; otherwise, <c>false</c>.</returns>
        public static bool operator ==(Matrix4 m1, Matrix4 m2) =>
            m1.C1 == m2.C1 && m1.C2 == m2.C2 && m1.C3 == m2.C3 && m1.C4 == m2.C4;

        /// <summary>
        /// Returns a flag indicating whether two specified matrices are unequal.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns><c>true</c> if the matrices are unequal; otherwise, <c>false</c>.</returns>
        public static bool operator !=(Matrix4 m1, Matrix4 m2) =>
            m1.C1 != m2.C1 || m1.C2 != m2.C2 || m1.C3 != m2.C3 || m1.C4 != m2.C4;

        /// <summary>
        /// Overrides <see cref="System.Object.Equals(object)" />.
        /// </summary>
        public override bool Equals(object obj) {
            if (obj is Matrix4 o) {
                return this == o;
            } else {
                return false;
            }
        }

        /// <summary>
        /// Overrides <see cref="System.Object.GetHashCode" />.
        /// </summary>
        public override int GetHashCode() => unchecked(
            C1.GetHashCode() ^ (C2.GetHashCode() * 6) ^
            (C3.GetHashCode() * 17) ^ (C4.GetHashCode() * 22)
        );

        /// <summary>
        /// Transposes the matrix.
        /// </summary>
        /// <param name="m">The matrix to transpose.</param>
        /// <returns>The transposed matrix.</returns>
        public static Matrix4 Transpose(Matrix4 m) {
            return new Matrix4()
            {
                C1 = new Vector4(m.C1.X, m.C2.X, m.C3.X, m.C4.X),
                C2 = new Vector4(m.C1.Y, m.C2.Y, m.C3.Y, m.C4.Y),
                C3 = new Vector4(m.C1.Z, m.C2.Z, m.C3.Z, m.C4.Z),
                C4 = new Vector4(m.C1.W, m.C2.W, m.C3.W, m.C4.W),
            };
        }

        /// <summary>
        /// Gets the identity matrix.
        /// </summary>
        /// <returns>The identity matrix.</returns>
        public static readonly Matrix4 Identity = new Matrix4()
        {
            C1 = new Vector4(1, 0, 0, 0),
            C2 = new Vector4(0, 1, 0, 0),
            C3 = new Vector4(0, 0, 1, 0),
            C4 = new Vector4(0, 0, 0, 1),
        };

        /// <summary>
        /// Creates a translation matrix with the specified displacement vector.
        /// </summary>
        /// <param name="position">The amount of translation.</param>
        /// <returns>The created translation matrix.</returns>
        public static Matrix4 CreateTranslation(Vector3 position) {
            return new Matrix4()
            {
                C1 = new Vector4(1, 0, 0, 0),
                C2 = new Vector4(0, 1, 0, 0),
                C3 = new Vector4(0, 0, 1, 0),
                C4 = new Vector4(position.X, position.Y, position.Z, 1),
            };
        }
    }
}