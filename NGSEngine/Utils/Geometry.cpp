#include "Geometry.h"

namespace ngs {

namespace {
    template <class T>
    inline void Matrix4Multiply(const T *a, const T *b, T *out)
    {
        out[0] = b[0] * a[0] + b[1] * a[4] + b[2] * a[8] + b[3] * a[12];
        out[1] = b[0] * a[1] + b[1] * a[5] + b[2] * a[9] + b[3] * a[13];
        out[2] = b[0] * a[2] + b[1] * a[6] + b[2] * a[10] + b[3] * a[14];
        out[3] = b[0] * a[3] + b[1] * a[7] + b[2] * a[11] + b[3] * a[15];

        out[4] = b[4] * a[0] + b[5] * a[4] + b[6] * a[8] + b[7] * a[12];
        out[5] = b[4] * a[1] + b[5] * a[5] + b[6] * a[9] + b[7] * a[13];
        out[6] = b[4] * a[2] + b[5] * a[6] + b[6] * a[10] + b[7] * a[14];
        out[7] = b[4] * a[3] + b[5] * a[7] + b[6] * a[11] + b[7] * a[15];

        out[8] = b[8] * a[0] + b[9] * a[4] + b[10] * a[8] + b[11] * a[12];
        out[9] = b[8] * a[1] + b[9] * a[5] + b[10] * a[9] + b[11] * a[13];
        out[10] = b[8] * a[2] + b[9] * a[6] + b[10] * a[10] + b[11] * a[14];
        out[11] = b[8] * a[3] + b[9] * a[7] + b[10] * a[11] + b[11] * a[15];

        out[12] = b[12] * a[0] + b[13] * a[4] + b[14] * a[8] + b[15] * a[12];
        out[13] = b[12] * a[1] + b[13] * a[5] + b[14] * a[9] + b[15] * a[13];
        out[14] = b[12] * a[2] + b[13] * a[6] + b[14] * a[10] + b[15] * a[14];
        out[15] = b[12] * a[3] + b[13] * a[7] + b[14] * a[11] + b[15] * a[15];
    }
}

template <class T>
BaseMatrix4<T>
BaseMatrix4<T>::MakeTranslate(T x, T y, T z)
{
    return BaseMatrix4{ 1, 0, 0, x, 0, 1, 0, y, 0, 0, 1, z, 0, 0, 0, 1 };
}

template <class T>
BaseMatrix4<T>
BaseMatrix4<T>::MakeScale(T x, T y, T z)
{
    return BaseMatrix4{ x, 0, 0, 0, 0, y, 0, 0, 0, 0, z, 0, 0, 0, 0, 1 };
}

template <class T>
BaseMatrix4<T>
BaseMatrix4<T>::MakeRotate(const BaseVector3D<T> &axis, T radians)
{
    const auto ax = axis.GetNormalized();
    const T c = std::cos(radians), s = std::sin(radians);
    const T ic = 1 - c, x = ax.x, y = ax.y, z = ax.z;
    return BaseMatrix4{ x * x * ic + c,
                        x * y * ic + z * s,
                        x * z * ic - y * s,
                        0,
                        x * y * ic - z * s,
                        y * y * ic + c,
                        y * z * ic + x * s,
                        0,
                        x * z * ic + y * s,
                        y * z * ic - x * s,
                        z * z * ic + c,
                        0,
                        0,
                        0,
                        0,
                        1 };
}

template <class T>
BaseMatrix4<T>
BaseMatrix4<T>::GetTransposed() const
{
    return BaseMatrix4{ m[0], m[4], m[8],  m[12], m[1], m[5], m[9],  m[13],
                        m[2], m[6], m[10], m[14], m[3], m[7], m[11], m[15] };
}

template <class T>
BaseMatrix4<T>
BaseMatrix4<T>::GetInversed() const
{
    // TODO: matrix inverse
    return MakeIdentity();
}

template <class T>
BaseMatrix4<T>
BaseMatrix4<T>::operator+(const BaseMatrix4<T> &o) const
{
    BaseMatrix4<T> ret = *this;
    ret += o;
    return ret;
}
template <class T>
BaseMatrix4<T>
BaseMatrix4<T>::operator-(const BaseMatrix4<T> &o) const
{
    BaseMatrix4<T> ret = *this;
    ret -= o;
    return ret;
}
template <class T>
BaseMatrix4<T> BaseMatrix4<T>::operator*(const BaseMatrix4<T> &o) const
{
    BaseMatrix4<T> ret;
    Matrix4Multiply(m.data(), o.m.data(), ret.m.data());
    return ret;
}
template <class T>
BaseMatrix4<T> &
BaseMatrix4<T>::operator+=(const BaseMatrix4<T> &o)
{
    for (std::size_t i = 0; i < 16; ++i) {
        m[i] += o.m[i];
    }
    return *this;
}
template <class T>
BaseMatrix4<T> &
BaseMatrix4<T>::operator-=(const BaseMatrix4<T> &o)
{
    for (std::size_t i = 0; i < 16; ++i) {
        m[i] -= o.m[i];
    }
    return *this;
}
template <class T>
BaseMatrix4<T> &
BaseMatrix4<T>::operator*=(const BaseMatrix4<T> &o)
{
    *this = *this * o;
    return *this;
}

template struct BaseMatrix4<float>;
template struct BaseMatrix4<double>;

}
