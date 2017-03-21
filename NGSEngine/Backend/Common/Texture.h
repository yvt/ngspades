#pragma once

#include <NGSCore.h>

namespace ngs {

class Texture : public RefCounted
{
protected:
    Texture() {}
    virtual ~Texture() {}
};
}
