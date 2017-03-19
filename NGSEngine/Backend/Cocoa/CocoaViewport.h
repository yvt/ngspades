#pragma once

#include <Backend/Common/IViewport.idl>

namespace ngs {

class CocoaViewport : public IViewport
{
public:
    NS_DECL_ISUPPORTS
    NS_DECL_IVIEWPORT

    CocoaViewport();

private:
    ~CocoaViewport();
};
}
