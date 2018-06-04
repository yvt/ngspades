//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System.Numerics;
using System.Runtime.InteropServices;

namespace Ngs.Engine {
    /// <summary>
    /// Represents a 3x3 matrix.
    /// </summary>
    /// <remarks>
    /// The components of matrices are arranged in a column-major order.
    /// </remarks>
    [StructLayout(LayoutKind.Sequential)]
    public struct Matrix3 {
        /// <summary>
        /// The first column.
        /// </summary>
        public Vector3 C1;

        /// <summary>
        /// The second column.
        /// </summary>
        public Vector3 C2;

        /// <summary>
        /// The third column.
        /// </summary>
        public Vector3 C3;

        /// <summary>
        /// Creates a 3x3 matrix from the specified components.
        /// </summary>
        /// <param name="m11">The first element in the first row.</param>
        /// <param name="m12">The second element in the first row.</param>
        /// <param name="m13">The third element in the first row.</param>
        /// <param name="m21">The first element in the second row.</param>
        /// <param name="m22">The second element in the second row.</param>
        /// <param name="m23">The third element in the second row.</param>
        /// <param name="m31">The first element in the third row.</param>
        /// <param name="m32">The second element in the third row.</param>
        /// <param name="m33">The third element in the third row.</param>
        /// <remarks>
        /// The components are specified in a row-major order in the parameters
        /// of this constructor.
        /// </remarks>
        public Matrix3(
            float m11, float m12, float m13,
            float m21, float m22, float m23,
            float m31, float m32, float m33
        ) {
            C1 = new Vector3(m11, m21, m31);
            C2 = new Vector3(m12, m22, m32);
            C3 = new Vector3(m13, m23, m33);
        }

        /// <summary>
        /// Adds two matricies.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns>The addition result.</returns>
        public static Matrix3 operator +(Matrix3 m1, Matrix3 m2) => new Matrix3()
        {
            C1 = m1.C1 + m2.C1,
            C2 = m1.C2 + m2.C2,
            C3 = m1.C3 + m2.C3,
        };

        /// <summary>
        /// Subtracts the second matrix from the first one.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns>The subtraction result.</returns>
        public static Matrix3 operator -(Matrix3 m1, Matrix3 m2) => new Matrix3()
        {
            C1 = m1.C1 - m2.C1,
            C2 = m1.C2 - m2.C2,
            C3 = m1.C3 - m2.C3,
        };

        /// <summary>
        /// Multiplies two matricies.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns>The multiplication result.</returns>
        public static Matrix3 operator *(Matrix3 m1, Matrix3 m2) {
            var mt = Matrix3.Transpose(m1);
            // TODO: optimize this to (maybe) eliminate the use of dot products?

            return new Matrix3()
            {
                C1 = new Vector3(
                        Vector3.Dot(mt.C1, m2.C1),
                        Vector3.Dot(mt.C2, m2.C1),
                        Vector3.Dot(mt.C3, m2.C1)
                    ),
                C2 = new Vector3(
                        Vector3.Dot(mt.C1, m2.C2),
                        Vector3.Dot(mt.C2, m2.C2),
                        Vector3.Dot(mt.C3, m2.C2)
                    ),
                C3 = new Vector3(
                        Vector3.Dot(mt.C1, m2.C3),
                        Vector3.Dot(mt.C2, m2.C3),
                        Vector3.Dot(mt.C3, m2.C3)
                    ),
            };
        }

        /// <summary>
        /// Multiplies a matrix and vector.
        /// </summary>
        /// <param name="m">The matrix used as the left operand.</param>
        /// <param name="v">The vector used as the right operand.</param>
        /// <returns>The multiplication result.</returns>
        public static Vector3 operator *(Matrix3 m, Vector3 v) =>
            m.C1 * v.X + m.C2 * v.Y + m.C3 * v.Z;

        /// <summary>
        /// Transforms a two-dimensional point represented by a specified vector by this
        /// transformation matrix.
        /// </summary>
        /// <param name="v">The vector to transform.</param>
        /// <returns>The transformed vector.</returns>
        public Vector2 TransformPoint(Vector2 v) {
            var factor = this * v.Extend(1);
            factor *= 1 / factor.Z;
            return factor.Truncate();
        }

        /// <summary>
        /// Transforms a two-dimensional (directional) vector by this transformation matrix.
        /// </summary>
        /// <param name="v">The vector to transform.</param>
        /// <returns>The transformed vector.</returns>
        public Vector2 TransformVector(Vector2 v) => (this * v.Extend(0)).Truncate();

        /// <summary>
        /// Returns a flag indicating whether two specified matrices are equal.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns><c>true</c> if the matrices are equal; otherwise, <c>false</c>.</returns>
        public static bool operator ==(Matrix3 m1, Matrix3 m2) =>
            m1.C1 == m2.C1 && m1.C2 == m2.C2 && m1.C3 == m2.C3;

        /// <summary>
        /// Returns a flag indicating whether two specified matrices are unequal.
        /// </summary>
        /// <param name="m1">The first matrix.</param>
        /// <param name="m2">The second matrix.</param>
        /// <returns><c>true</c> if the matrices are unequal; otherwise, <c>false</c>.</returns>
        public static bool operator !=(Matrix3 m1, Matrix3 m2) =>
            m1.C1 != m2.C1 || m1.C2 != m2.C2 || m1.C3 != m2.C3;

        /// <summary>
        /// Overrides <see cref="System.Object.Equals(object)" />.
        /// </summary>
        public override bool Equals(object obj) {
            if (obj is Matrix3 o) {
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
            (C3.GetHashCode() * 17)
        );

        /// <summary>
        /// Transposes the matrix.
        /// </summary>
        /// <param name="m">The matrix to transpose.</param>
        /// <returns>The transposed matrix.</returns>
        public static Matrix3 Transpose(Matrix3 m) => new Matrix3()
        {
            C1 = new Vector3(m.C1.X, m.C2.X, m.C3.X),
            C2 = new Vector3(m.C1.Y, m.C2.Y, m.C3.Y),
            C3 = new Vector3(m.C1.Z, m.C2.Z, m.C3.Z),
        };

        /// <summary>
        /// Gets the identity matrix.
        /// </summary>
        /// <returns>The identity matrix.</returns>
        public static readonly Matrix3 Identity = new Matrix3()
        {
            C1 = new Vector3(1, 0, 0),
            C2 = new Vector3(0, 1, 0),
            C3 = new Vector3(0, 0, 1),
        };

        /// <summary>
        /// Inverts the specified matrix.
        /// </summary>
        /// <param name="m">The matrix to invert.</param>
        /// <param name="result">The inverted matrix.</param>
        /// <returns><c>true</c> if the inversion was successfully; otherwise, <c>false</c>.
        /// </returns>
        public static bool Invert(Matrix3 m, out Matrix3 result) {
            // TODO: Optimize `Matrix3.Invert`
            if (Matrix4.Invert(new Matrix4()
            {
                C1 = m.C1.Extend(0),
                C2 = m.C2.Extend(0),
                C3 = m.C3.Extend(0),
                C4 = new Vector4(0, 0, 0, 1),
            }, out var result4)) {
                result = new Matrix3
                {
                    C1 = result4.C1.Truncate(),
                    C2 = result4.C2.Truncate(),
                    C3 = result4.C3.Truncate()
                };
                return true;
            } else {
                result = default;
                return false;
            }
        }
    }
}
