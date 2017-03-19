#include <algorithm>

#ifdef WIN32
#include <Windows.h>
#include <OleAuto.h>
#endif

#include "BString.h"

namespace ngs {

// TODO: we should use SysAllocStringLen and SysFreeString on Windows platform

void
BString::Free() noexcept
{
#ifdef WIN32
	SysFreeString(reinterpret_cast<BSTR>(this));
#else
    std::free(GetMemoryBlock());
#endif
}

BStringRef
BString::Allocate(std::size_t length) noexcept
{
    if (length >= 0x80000000) {
        return {};
    }

#ifdef WIN32
	BSTR bstr = SysAllocStringLen(nullptr, static_cast<UINT>(length));
	if (!bstr) {
		return {};
	}

	return { reinterpret_cast<BString *>(bstr), Deleter{} };
#else
    void *mem = std::malloc(length * 2 + 6); // data + prefix + terminator
    if (!mem) {
        return {};
    }

    *reinterpret_cast<std::uint32_t *>(mem) = static_cast<std::uint32_t>(length * 2);
    reinterpret_cast<char16_t *>(mem)[2 + length] = 0; // terminator (null char)
    return { reinterpret_cast<BString *>(reinterpret_cast<std::uint32_t *>(mem) + 1), Deleter{} };
#endif
}

BStringRef
BString::Create(U16StringView str) noexcept
{
    auto ret = Allocate(str.size());
    std::copy(str.begin(), str.end(), ret->GetData());
    return ret;
}

}
