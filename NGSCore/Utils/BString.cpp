#include <algorithm>

#ifdef WIN32
#include <Windows.h>
#include <OleAuto.h>
#endif

#include "BString.h"

namespace ngs {

namespace {
    const BStringVTable g_bstringVTable = {
        [](BString *mem) {
            mem->~BString();
            operator delete(mem);
        }
    };
}

BString::BString(std::int32_t length) :
    m_vtable{g_bstringVTable},
    m_length{length}
{
    GetData()[length] = 0; // terminator
}

BStringRef
BString::Allocate(std::size_t length) noexcept
{
    if (length >= 0x40000000) {
        return {};
    }

    void *mem = operator new(sizeof(BString) + length);
    if (!mem) {
        return {};
    }

    BString *s = new (mem) BString(length);

    return { s, Deleter{} };
}

BStringRef
BString::Create(StringView str) noexcept
{
    auto ret = Allocate(str.size());
    std::copy(str.begin(), str.end(), ret->GetData());
    return ret;
}

}
