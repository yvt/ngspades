#pragma once

#include <atomic>

namespace ngs {
class RefCounted
{
public:
    RefCounted();

    void AddRef();
    void Release();

protected:
    virtual ~RefCounted();

private:
    std::atomic<std::int32_t> m_refCount;
};
}
