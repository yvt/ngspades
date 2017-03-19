#pragma once

#include <cassert>
#include <cstdint>
#include <cstdlib>
#include <memory>
#include <string>

#if __has_include(<string_view>)
#include <string_view> // C++17
namespace ngs {
using U16StringView = std::u16string_view;
}
#else
// support legacy compiler
#include <experimental/string_view>
namespace ngs {
using U16StringView = std::experimental::u16string_view;
}
#endif

namespace ngs {

/**
 * Something like COM's BSTR
 */
class BString
{
public:
    struct Deleter
    {
        void operator()(BString *ptr) const noexcept { ptr->Free(); }
    };

    using Ref = std::unique_ptr<BString, BString::Deleter>;

    inline void *GetMemoryBlock() noexcept { return reinterpret_cast<std::uint32_t *>(this) - 1; }
    inline const void *GetMemoryBlock() const noexcept
    {
        return reinterpret_cast<const std::uint32_t *>(this) - 1;
    }

    void Free() noexcept;

    inline Ref Clone() const noexcept { return Create(*this); }

    inline char16_t *GetData() noexcept
    {
        return reinterpret_cast<char16_t *>(GetMemoryBlock()) + 2;
    }
    inline const char16_t *GetData() const noexcept
    {
        return reinterpret_cast<const char16_t *>(GetMemoryBlock()) + 2;
    }
    inline operator char16_t *() noexcept { return GetData(); }
    inline operator const char16_t *() const noexcept { return GetData(); }

    inline std::size_t GetLength() const noexcept
    {
        return *reinterpret_cast<const std::uint32_t *>(GetMemoryBlock());
    }

    inline U16StringView GetView() const noexcept { return { GetData(), GetLength() }; }
    inline operator U16StringView() const noexcept { return GetView(); }

    static Ref Allocate(std::size_t length) noexcept;
    static Ref Create(U16StringView str) noexcept;
    template<std::size_t length>
    static Ref Create(const char16_t (&str)[length]) noexcept
    {
        return Create(U16StringView{ str, length });
    }

private:
    BString() = delete;
    ~BString() = delete;
};

using BStringRef = BString::Ref;

/*

    Little notes on the usage of `BString`:

    - `BString` is an unsized object which is created by `BString::Allocate` and
      freed by `BString::Free`. You can't instantiate `BString` directly.
    - `BString *` is the pointer to the UTF-16 string data. You can use `BString *`
      to pass strings during COM calls.
    - `BStringRef` is a smart pointer for `BString`. Leaving the scope automatically
      frees the held `BString *` unless you `BStringRef::release()` it.
    - An `in` parameter accepts `const BString *`. It is owned by the caller and
      you can't `Free` it nor modify its contents.
    - An `out` parameter accpets `BString **`. After the function returns it's
      `BString *` considered owned by the caller. This means you cannot assign
      a `BString *` owned by you to it. You need to clone it first.
    - An `inout` parameter - I don't know how this exactly works.

    Examples:

    void ExampleFunction(const BString *in, BString **out)
    {
        *out = BString::Create<>(u"SomeRandomString").release();
    }

 */

}
