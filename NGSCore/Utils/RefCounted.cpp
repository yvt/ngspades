#include "RefCounted.h"

namespace ngs {
RefCounted::RefCounted() : m_refCount{ 1 }
{
}

void
RefCounted::AddRef()
{
    ++m_refCount;
}

void
RefCounted::Release()
{
    if ((--m_refCount) == 0) {
        delete this;
    }
}

RefCounted::~RefCounted()
{
}
}
