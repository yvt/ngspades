#include "CocoaViewport.h"

namespace ngs {

NS_IMPL_ISUPPORTS(CocoaViewport, IViewport)

CocoaViewport::CocoaViewport()
{
    /* member initializers and constructor code */
}

CocoaViewport::~CocoaViewport()
{
    /* destructor code */
}

/* void AddListener (in IViewportListener listener); */
NS_IMETHODIMP
CocoaViewport::AddListener(IViewportListener *listener)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* void RemoveListener (in IViewportListener listener); */
NS_IMETHODIMP
CocoaViewport::RemoveListener(IViewportListener *listener)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* readonly attribute long VideoWidth; */
NS_IMETHODIMP
CocoaViewport::GetVideoWidth(int32_t *aVideoWidth)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* readonly attribute long VideoHeight; */
NS_IMETHODIMP
CocoaViewport::GetVideoHeight(int32_t *aVideoHeight)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* readonly attribute ngsFullScreenMode FullScreenMode; */
NS_IMETHODIMP
CocoaViewport::GetFullScreenMode(ngs::FullScreenMode *aFullScreenMode)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* readonly attribute float DevicePixelRatio; */
NS_IMETHODIMP
CocoaViewport::GetDevicePixelRatio(float *aDevicePixelRatio)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* void SetVideoMode (in long videoWidth, in long videoHeight, in ngsFullScreenMode fullScreenMode,
 * in boolean useNativePixelRatio); */
NS_IMETHODIMP
CocoaViewport::SetVideoMode(int32_t videoWidth, int32_t videoHeight,
                            ngs::FullScreenMode fullScreenMode, bool useNativePixelRatio)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* attribute boolean EnableTextInput; */
NS_IMETHODIMP
CocoaViewport::GetEnableTextInput(bool *aEnableTextInput)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}
NS_IMETHODIMP
CocoaViewport::SetEnableTextInput(bool aEnableTextInput)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* attribute ngsBox2D TextInputRectangle; */
NS_IMETHODIMP
CocoaViewport::GetTextInputRectangle(ngs::Box2D *aTextInputRectangle)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}
NS_IMETHODIMP
CocoaViewport::SetTextInputRectangle(ngs::Box2D aTextInputRectangle)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}
}