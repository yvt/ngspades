#pragma once

#include <cassert>
#include <cstdint>
#include <cstdlib>
#include <memory>
#include <string>

#if defined(_MSC_VER) || __has_include(<string_view>)
#include <string_view> // C++17
namespace ngs {
using StringView = std::string_view;
}
#else
// support legacy compiler
#include <experimental/string_view>
namespace ngs {
using StringView = std::experimental::string_view;
}
#endif

namespace ngs {

class BString;

struct BStringVTable
{
    void (*Destruct)(BString *);
};

/**
 * Not alike COM's BSTR
 */
class BString
{
public:
    struct Deleter
    {
        void operator()(BString *ptr) const noexcept { ptr->Free(); }
    };

    using Ref = std::unique_ptr<BString, BString::Deleter>;

    void Free() noexcept { m_vtable.Destruct(this); }

    inline Ref Clone() const noexcept { return Create(*this); }

    inline char *GetData() noexcept { return m_data; }
    inline const char *GetData() const noexcept { return m_data; }
    inline operator char *() noexcept { return GetData(); }
    inline operator const char *() const noexcept { return GetData(); }

    inline std::size_t GetLength() const noexcept { return static_cast<std::size_t>(m_length); }

    inline StringView GetView() const noexcept { return { GetData(), GetLength() }; }
    inline operator StringView() const noexcept { return GetView(); }

    static Ref Allocate(std::size_t length) noexcept;
    static Ref Create(StringView str) noexcept;
    template<std::size_t length>
    static Ref Create(const char (&str)[length]) noexcept
    {
        return Create(StringView{ str, length - 1 });
    }

private:
    BString(std::int32_t length);
    BString(const BString &) = delete;
    void operator=(const BString &) = delete;

    const BStringVTable &m_vtable;
    std::int32_t const m_length;
    char m_data[1];
};

using BStringRef = BString::Ref;

/*

    Little notes on the usage of `BString`:

    - `BString` is an unsized object which is created by `BString::Allocate` and
      freed by `BString::Free`. You can't instantiate `BString` directly.
    - `BString *` is the pointer to the UTF-8 string data. You can use `BString *`
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
        *out = BString::Create<>("SomeRandomString").release();
    }

 */
}
