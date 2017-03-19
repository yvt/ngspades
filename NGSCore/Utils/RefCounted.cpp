#include "RefCounted.h"

namespace ngs {
RefCounted::RefCounted()
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
