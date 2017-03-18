#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <type_traits>

namespace ngs {

template <class T>
struct BaseVector2D
{
    T x, y;

    using Self = BaseVector2D<T>;
    using RefRemoved = BaseVector2D<typename std::remove_reference<T>::type>;

    BaseVector2D() = default;
    constexpr inline BaseVector2D(T x, T y) : x(x), y(y) {}
    explicit constexpr inline BaseVector2D(T v) : x(v), y(v) {}
    template <class S>
    explicit constexpr inline BaseVector2D(const BaseVector2D<S> &o)
      : x(static_cast<T>(o.x)), y(static_cast<T>(o.y))
    {
    }
    constexpr inline BaseVector2D(const BaseVector2D<T &> o) : x(o.x), y(o.y) {}

    inline BaseVector2D &operator=(const RefRemoved &o) const
    {
        x = o.x;
        y = o.y;
        return *this;
    }

    constexpr inline RefRemoved operator+(const RefRemoved &o) const
    {
        return RefRemoved(x + o.x, y + o.y);
    }
    constexpr inline RefRemoved operator-(const RefRemoved &o) const
    {
        return RefRemoved(x - o.x, y - o.y);
    }
    constexpr inline RefRemoved operator*(const RefRemoved &o) const
    {
        return RefRemoved(x * o.x, y * o.y);
    }
    constexpr inline RefRemoved operator/(const RefRemoved &o) const
    {
        return RefRemoved(x / o.x, y / o.y);
    }
    constexpr inline RefRemoved operator+(T o) const { return RefRemoved(x + o, y + o); }
    constexpr inline RefRemoved operator-(T o) const { return RefRemoved(x - o, y - o); }
    constexpr inline RefRemoved operator*(T o) const { return RefRemoved(x * o, y * o); }
    constexpr inline RefRemoved operator/(T o) const { return RefRemoved(x / o, y / o); }
    inline Self &operator+=(const RefRemoved &o) const
    {
        x += o.x;
        y += o.y;
        return *this;
    }
    inline Self &operator-=(const RefRemoved &o) const
    {
        x -= o.x;
        y -= o.y;
        return *this;
    }
    inline Self &operator*=(const RefRemoved &o) const
    {
        x *= o.x;
        y *= o.y;
        return *this;
    }
    inline Self &operator/=(const RefRemoved &o) const
    {
        x /= o.x;
        y /= o.y;
        return *this;
    }
    inline Self &operator+=(T o) const
    {
        x += o;
        y += o;
        return *this;
    }
    inline Self &operator-=(T o) const
    {
        x -= o;
        y -= o;
        return *this;
    }
    inline Self &operator*=(T o) const
    {
        x *= o;
        y *= o;
        return *this;
    }
    inline Self &operator/=(T o) const
    {
        x /= o;
        y /= o;
        return *this;
    }

    constexpr inline RefRemoved operator-() const { return RefRemoved(-x, -y); }

    constexpr inline bool operator==(const RefRemoved &o) const { return x == o.x && y == o.y; }
    constexpr inline bool operator!=(const RefRemoved &o) const { return x != o.x || y != o.y; }

    constexpr inline T GetLengthSquared() const { return x * x + y * y; }

    constexpr inline T GetManhattanLength() const { return std::abs(x) + std::abs(y); }

    constexpr inline T GetChebyshevLength() const { return std::max({ std::abs(x), std::abs(y) }); }

    inline T GetLength() const { return std::sqrt(this->GetLengthSquared()); }

    inline RefRemoved GetNormalized() const { return *this * (1 / GetLength()); }

    inline RefRemoved GetPerpendicularVector() const { return RefRemoved(-y, x); }

    inline void Normalize() { *this *= 1 / GetLength(); }

    friend inline RefRemoved Round(const BaseVector2D &v)
    {
        return RefRemoved(std::round(v.x), std::round(v.y));
    }

    friend inline RefRemoved Floor(const BaseVector2D &v)
    {
        return RefRemoved(std::floor(v.x), std::floor(v.y));
    }

    friend inline RefRemoved Ceil(const BaseVector2D &v)
    {
        return RefRemoved(std::ceil(v.x), std::ceil(v.y));
    }

    friend constexpr inline T Dot(const Self &a, const RefRemoved &b)
    {
        return a.x * b.x + a.y * b.y;
    }
};

template <class T>
struct BaseVector3D
{
    T x, y, z;

    using Self = BaseVector3D<T>;
    using RefRemoved = BaseVector3D<typename std::remove_reference<T>::type>;

    BaseVector3D() = default;
    constexpr inline BaseVector3D(T x, T y, T z) : x(x), y(y), z(z) {}
    explicit constexpr inline BaseVector3D(T v) : x(v), y(v), z(v) {}
    template <class S>
    explicit constexpr inline BaseVector3D(const BaseVector3D<S> &o)
      : x(static_cast<T>(o.x)), y(static_cast<T>(o.y)), z(static_cast<T>(o.z))
    {
    }
    constexpr inline BaseVector3D(const BaseVector3D<T &> o) : x(o.x), y(o.y), z(o.z) {}

    inline BaseVector3D &operator=(const RefRemoved &o) const
    {
        x = o.x;
        y = o.y;
        z = o.z;
        return *this;
    }

    constexpr inline RefRemoved operator+(const RefRemoved &o) const
    {
        return RefRemoved(x + o.x, y + o.y, z + o.z);
    }
    constexpr inline RefRemoved operator-(const RefRemoved &o) const
    {
        return RefRemoved(x - o.x, y - o.y, z - o.z);
    }
    constexpr inline RefRemoved operator*(const RefRemoved &o) const
    {
        return RefRemoved(x * o.x, y * o.y, z * o.z);
    }
    constexpr inline RefRemoved operator/(const RefRemoved &o) const
    {
        return RefRemoved(x / o.x, y / o.y, z / o.z);
    }
    constexpr inline RefRemoved operator+(T o) const { return RefRemoved(x + o, y + o, z + o); }
    constexpr inline RefRemoved operator-(T o) const { return RefRemoved(x - o, y - o, z - o); }
    constexpr inline RefRemoved operator*(T o) const { return RefRemoved(x * o, y * o, z * o); }
    constexpr inline RefRemoved operator/(T o) const { return RefRemoved(x / o, y / o, z / o); }
    inline Self &operator+=(const RefRemoved &o) const
    {
        x += o.x;
        y += o.y;
        z += o.z;
        return *this;
    }
    inline Self &operator-=(const RefRemoved &o) const
    {
        x -= o.x;
        y -= o.y;
        z -= o.z;
        return *this;
    }
    inline Self &operator*=(const RefRemoved &o) const
    {
        x *= o.x;
        y *= o.y;
        z *= o.z;
        return *this;
    }
    inline Self &operator/=(const RefRemoved &o) const
    {
        x /= o.x;
        y /= o.y;
        z /= o.z;
        return *this;
    }
    inline Self &operator+=(T o) const
    {
        x += o;
        y += o;
        z += o;
        return *this;
    }
    inline Self &operator-=(T o) const
    {
        x -= o;
        y -= o;
        z -= o;
        return *this;
    }
    inline Self &operator*=(T o) const
    {
        x *= o;
        y *= o;
        z *= o;
        return *this;
    }
    inline Self &operator/=(T o) const
    {
        x /= o;
        y /= o;
        z /= o;
        return *this;
    }

    constexpr inline RefRemoved operator-() const { return RefRemoved(-x, -y, -z); }

    constexpr inline bool operator==(const RefRemoved &o) const
    {
        return x == o.x && y == o.y && z == o.z;
    }
    constexpr inline bool operator!=(const RefRemoved &o) const
    {
        return x != o.x || y != o.y || z != o.z;
    }

    constexpr inline T GetLengthSquared() const { return x * x + y * y + z * z; }

    constexpr inline T GetManhattanLength() const
    {
        return std::abs(x) + std::abs(y) + std::abs(z);
    }

    constexpr inline T GetChebyshevLength() const
    {
        return std::max({ std::abs(x), std::abs(y), std::abs(z) });
    }

    inline T GetLength() const { return std::sqrt(this->GetLengthSquared()); }

    inline RefRemoved GetNormalized() const { return *this * (1 / GetLength()); }

    inline void Normalize() { *this *= 1 / GetLength(); }

    friend inline RefRemoved Round(const BaseVector3D &v)
    {
        return RefRemoved(std::round(v.x), std::round(v.y), std::round(v.z));
    }

    friend inline RefRemoved Floor(const BaseVector3D &v)
    {
        return RefRemoved(std::floor(v.x), std::floor(v.y), std::floor(v.z));
    }

    friend inline RefRemoved Ceil(const BaseVector3D &v)
    {
        return RefRemoved(std::ceil(v.x), std::ceil(v.y), std::ceil(v.z));
    }

    friend constexpr inline T Dot(const Self &a, const RefRemoved &b)
    {
        return a.x * b.x + a.y * b.y + a.z * b.z;
    }

    friend constexpr inline RefRemoved Cross(const Self &a, const RefRemoved &b)
    {
        return RefRemoved(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x);
    }
};

template <class T>
struct BaseVector4D
{
    T x, y, z, w;

    using Self = BaseVector4D<T>;
    using RefRemoved = BaseVector4D<typename std::remove_reference<T>::type>;

    BaseVector4D() = default;
    constexpr inline BaseVector4D(T x, T y, T z, T w) : x(x), y(y), z(z), w(w) {}
    explicit constexpr inline BaseVector4D(T v) : x(v), y(v), z(v), w(v) {}
    template <class S>
    explicit constexpr inline BaseVector4D(const BaseVector4D<S> &o)
      : x(static_cast<T>(o.x))
      , y(static_cast<T>(o.y))
      , z(static_cast<T>(o.z))
      , w(static_cast<T>(o.w))
    {
    }
    constexpr inline BaseVector4D(const BaseVector4D<T &> o) : x(o.x), y(o.y), z(o.z), w(o.w) {}

    inline BaseVector4D &operator=(const RefRemoved &o) const
    {
        x = o.x;
        y = o.y;
        z = o.z;
        w = o.w;
        return *this;
    }

    constexpr inline RefRemoved operator+(const RefRemoved &o) const
    {
        return RefRemoved(x + o.x, y + o.y, z + o.z, w + o.w);
    }
    constexpr inline RefRemoved operator-(const RefRemoved &o) const
    {
        return RefRemoved(x - o.x, y - o.y, z - o.z, w - o.w);
    }
    constexpr inline RefRemoved operator*(const RefRemoved &o) const
    {
        return RefRemoved(x * o.x, y * o.y, z * o.z, w * o.w);
    }
    constexpr inline RefRemoved operator/(const RefRemoved &o) const
    {
        return RefRemoved(x / o.x, y / o.y, z / o.z, w / o.w);
    }
    constexpr inline RefRemoved operator+(T o) const
    {
        return RefRemoved(x + o, y + o, z + o, w + o);
    }
    constexpr inline RefRemoved operator-(T o) const
    {
        return RefRemoved(x - o, y - o, z - o, w - o);
    }
    constexpr inline RefRemoved operator*(T o) const
    {
        return RefRemoved(x * o, y * o, z * o, w * o);
    }
    constexpr inline RefRemoved operator/(T o) const
    {
        return RefRemoved(x / o, y / o, z / o, w / o);
    }
    inline Self &operator+=(const RefRemoved &o) const
    {
        x += o.x;
        y += o.y;
        z += o.z;
        w += o.w;
        return *this;
    }
    inline Self &operator-=(const RefRemoved &o) const
    {
        x -= o.x;
        y -= o.y;
        z -= o.z;
        w -= o.w;
        return *this;
    }
    inline Self &operator*=(const RefRemoved &o) const
    {
        x *= o.x;
        y *= o.y;
        z *= o.z;
        w *= o.w;
        return *this;
    }
    inline Self &operator/=(const RefRemoved &o) const
    {
        x /= o.x;
        y /= o.y;
        z /= o.z;
        w /= o.w;
        return *this;
    }
    inline Self &operator+=(T o) const
    {
        x += o;
        y += o;
        z += o;
        w += o;
        return *this;
    }
    inline Self &operator-=(T o) const
    {
        x -= o;
        y -= o;
        z -= o;
        w -= o;
        return *this;
    }
    inline Self &operator*=(T o) const
    {
        x *= o;
        y *= o;
        z *= o;
        w *= o;
        return *this;
    }
    inline Self &operator/=(T o) const
    {
        x /= o;
        y /= o;
        z /= o;
        w /= o;
        return *this;
    }

    constexpr inline RefRemoved operator-() const { return RefRemoved(-x, -y, -z, -w); }

    constexpr inline bool operator==(const RefRemoved &o) const
    {
        return x == o.x && y == o.y && z == o.z && w == o.w;
    }
    constexpr inline bool operator!=(const RefRemoved &o) const
    {
        return x != o.x || y != o.y || z != o.z || w != o.w;
    }

    constexpr inline T GetLengthSquared() const { return x * x + y * y + z * z + w * w; }

    constexpr inline T GetManhattanLength() const
    {
        return std::abs(x) + std::abs(y) + std::abs(z) + std::abs(w);
    }

    constexpr inline T GetChebyshevLength() const
    {
        return std::max({ std::abs(x), std::abs(y), std::abs(z), std::abs(w) });
    }

    inline T GetLength() const { return std::sqrt(this->GetLengthSquared()); }

    inline RefRemoved GetNormalized() const { return *this * (1 / GetLength()); }

    inline void Normalize() { *this *= 1 / GetLength(); }

    inline BaseVector3D<T &> GetXYZ() { return { x, y, z }; }
    inline BaseVector3D<T> GetXYZ() const { return { x, y, z }; }

    friend inline RefRemoved Round(const Self &v)
    {
        return RefRemoved(std::round(v.x), std::round(v.y), std::round(v.z), std::round(v.w));
    }

    friend inline RefRemoved Floor(const Self &v)
    {
        return RefRemoved(std::floor(v.x), std::floor(v.y), std::floor(v.z), std::floor(v.w));
    }

    friend inline RefRemoved Ceil(const Self &v)
    {
        return RefRemoved(std::ceil(v.x), std::ceil(v.y), std::ceil(v.z), std::ceil(v.w));
    }

    friend constexpr inline T Dot(const Self &a, const RefRemoved &b)
    {
        return a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
    }
};

using IntVector2D = BaseVector2D<int32_t>;
using IntVector3D = BaseVector3D<int32_t>;
using IntVector4D = BaseVector4D<int32_t>;

using Vector2D = BaseVector2D<float>;
using Vector3D = BaseVector3D<float>;
using Vector4D = BaseVector4D<float>;

using DVector2D = BaseVector2D<double>;
using DVector3D = BaseVector3D<double>;
using DVector4D = BaseVector4D<double>;

using IntVector2DRef = BaseVector2D<int32_t &>;
using IntVector3DRef = BaseVector3D<int32_t &>;
using IntVector4DRef = BaseVector4D<int32_t &>;

using Vector2DRef = BaseVector2D<float &>;
using Vector3DRef = BaseVector3D<float &>;
using Vector4DRef = BaseVector4D<float &>;

using DVector2DRef = BaseVector2D<double &>;
using DVector3DRef = BaseVector3D<double &>;
using DVector4DRef = BaseVector4D<double &>;

/** plane n dot x + w = 0 */
template <class T>
struct BasePlane2D
{
    BaseVector2D<T> n;
    float w;

    BasePlane2D() = default;
    constexpr inline BasePlane2D(const BaseVector2D<T> &n, float w) : n(n), w(w) {}

    static inline BasePlane2D FromPoints(const BaseVector2D<T> &a, const BaseVector2D<T> &b)
    {
        auto n = (b - a).GetPerpendicularVector().GetNormalized();
        return BasePlane2D(n, -Dot(a, n));
    }

    constexpr inline float GetSignedDistanceTo(const BaseVector2D<T> &v) const
    {
        return Dot(v, n) + w;
    }

    constexpr inline BaseVector2D<T> ProjectPoint(const BaseVector2D<T> &v) const
    {
        return v - n * GetSignedDistanceTo(v);
    }

    constexpr inline BasePlane2D GetFlipped() const { return BasePlane2D(-n, -w); }
};

using Plane2D = BasePlane2D<float>;
using DPlane2D = BasePlane2D<double>;

template <class T>
struct BaseMatrix4
{
    /** Elements are stored in the column-major order. */
    std::array<T, 16> m;

    BaseMatrix4() = default;
    constexpr inline BaseMatrix4(T m00, T m01, T m02, T m03, T m10, T m11, T m12, T m13, T m20,
                                 T m21, T m22, T m23, T m30, T m31, T m32, T m33)
      : m{ m00, m10, m20, m30, m01, m11, m21, m31, m02, m12, m22, m32, m03, m13, m23, m33 }
    {
    }
    explicit inline BaseMatrix4(const T *elements)
    {
        std::copy(elements, elements + 16, m.begin());
    }
    explicit constexpr inline BaseMatrix4(const BaseVector4D<T> &v)
      : BaseMatrix4{ v.x, 0, 0, 0, 0, v.y, 0, 0, 0, 0, v.z, 0, 0, 0, 0, v.w }
    {
    }
    explicit constexpr inline BaseMatrix4(T v) : BaseMatrix4{ BaseVector4D<T>{ v } } {}

    static inline BaseMatrix4<T> MakeIdentity() { return BaseMatrix4{ T{ 1 } }; }
    static BaseMatrix4<T> MakeTranslate(T x, T y, T z);
    static inline BaseMatrix4<T> MakeTranslate(const BaseVector3D<T> &v)
    {
        return MakeTranslate(v.x, v.y, v.z);
    }
    static BaseMatrix4<T> MakeRotate(const BaseVector3D<T> &axis, T radians);
    static BaseMatrix4<T> MakeScale(T x, T y, T z);
    static inline BaseMatrix4<T> MakeScale(T uniformScale)
    {
        return MakeScale(uniformScale, uniformScale, uniformScale);
    }
    static inline BaseMatrix4<T> MakeScale(const BaseVector3D<T> &v)
    {
        return MakeScale(v.x, v.y, v.z);
    }

    BaseMatrix4<T> operator+(const BaseMatrix4<T> &o) const;
    BaseMatrix4<T> operator-(const BaseMatrix4<T> &o) const;
    BaseMatrix4<T> operator*(const BaseMatrix4<T> &o) const;
    BaseMatrix4<T> &operator+=(const BaseMatrix4<T> &o);
    BaseMatrix4<T> &operator-=(const BaseMatrix4<T> &o);
    BaseMatrix4<T> &operator*=(const BaseMatrix4<T> &o);

    BaseMatrix4<T> GetTransposed() const;
    BaseMatrix4<T> GetInversed() const;

    template <int N>
    inline BaseVector4D<T &> GetColumn()
    {
        static_assert(N >= 0 && N < 4, "bad column index");
        return { m[N], m[N * 4 + 1], m[N * 4 + 2], m[N * 4 + 3] };
    }
    template <int N>
    inline BaseVector4D<T> GetColumn() const
    {
        static_assert(N >= 0 && N < 4, "bad column index");
        return { m[N], m[N * 4 + 1], m[N * 4 + 2], m[N * 4 + 3] };
    }
    inline std::array<BaseVector4D<T &>, 4> GetColumns()
    {
        return { GetColumn<0>(), GetColumn<1>(), GetColumn<2>(), GetColumn<3>() };
    }
    inline std::array<BaseVector4D<T>, 4> GetColumns() const
    {
        return { GetColumn<0>(), GetColumn<1>(), GetColumn<2>(), GetColumn<3>() };
    }

    template <int N>
    inline BaseVector4D<T &> GetRow()
    {
        static_assert(N >= 0 && N < 4, "bad row index");
        return { m[N], m[N + 4], m[N + 8], m[N + 12] };
    }
    template <int N>
    inline BaseVector4D<T> GetRow() const
    {
        static_assert(N >= 0 && N < 4, "bad row index");
        return { m[N], m[N + 4], m[N + 8], m[N + 12] };
    }
    inline std::array<BaseVector4D<T &>, 4> GetRows()
    {
        return { GetRow<0>(), GetRow<1>(), GetRow<2>(), GetRow<3>() };
    }
    inline std::array<BaseVector4D<T>, 4> GetRows() const
    {
        return { GetRow<0>(), GetRow<1>(), GetRow<2>(), GetRow<3>() };
    }
};

using Matrix4 = BaseMatrix4<float>;
using DMatrix4 = BaseMatrix4<double>;
}
