#pragma once

#include <atomic>
#include <cstdint>

namespace ngs {
class RefCounted
{
public:
    void AddRef();
    void Release();

protected:
    RefCounted();
    virtual ~RefCounted();

private:
    std::atomic<std::int32_t> m_refCount;
};
}
