#pragma once

#include <type_traits>

#include <BString.h>
#include <RefCounted.h>
#include <mozilla/RefPtr.h>

// namespace unification
namespace ngs {
template<class T>
using RefPtr = ::RefPtr<T>;
}

// various utilities
namespace ngs {
template<class T, class TIn>
inline T
Cast(TIn &&value)
{
    return static_cast<T>(value);
}
}

/**
 * Defines various operators so the specified `enum class` type
 * can be used to hold flags.
 *
 * Example:
 *
 *     enum class MyFlags { A = 1 << 0, B = 1 << 1 };
 *     NGS_DEFINE_FLAGS(MyFlags)
 *
 */
#define NGS_DEFINE_FLAGS(FlagsType)                                                                \
    inline FlagsType operator|(FlagsType a, FlagsType b)                                           \
    {                                                                                              \
        using T = std::underlying_type_t<FlagsType>;                                               \
        return Cast<FlagsType>(Cast<T>(a) | Cast<T>(b));                                           \
    }                                                                                              \
    inline FlagsType operator&(FlagsType a, FlagsType b)                                           \
    {                                                                                              \
        using T = std::underlying_type_t<FlagsType>;                                               \
        return Cast<FlagsType>(Cast<T>(a) & Cast<T>(b));                                           \
    }                                                                                              \
    inline FlagsType &operator|=(FlagsType &a, FlagsType b)                                        \
    {                                                                                              \
        a = a | b;                                                                                 \
        return a;                                                                                  \
    }                                                                                              \
    inline FlagsType &operator&=(FlagsType &a, FlagsType b)                                        \
    {                                                                                              \
        a = a & b;                                                                                 \
        return a;                                                                                  \
    }                                                                                              \
    explicit inline operator bool(FlagsType a)                                                     \
    {                                                                                              \
        using T = std::underlying_type_t<FlagsType>;                                               \
        return static_cast<T>(a) != 0;                                                             \
    }
