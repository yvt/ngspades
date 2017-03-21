#include "SDLViewport.h"

namespace ngs {

NS_IMPL_ISUPPORTS(SDLViewport, IViewport)

SDLViewport::SDLViewport()
{
    /* member initializers and constructor code */
}

SDLViewport::~SDLViewport()
{
    /* destructor code */
}

/* void AddListener (in IViewportListener listener); */
NS_IMETHODIMP
SDLViewport::AddListener(IViewportListener *listener)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* void RemoveListener (in IViewportListener listener); */
NS_IMETHODIMP
SDLViewport::RemoveListener(IViewportListener *listener)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* readonly attribute long VideoWidth; */
NS_IMETHODIMP
SDLViewport::GetVideoWidth(int32_t *aVideoWidth)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* readonly attribute long VideoHeight; */
NS_IMETHODIMP
SDLViewport::GetVideoHeight(int32_t *aVideoHeight)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* readonly attribute ngsFullScreenMode FullScreenMode; */
NS_IMETHODIMP
SDLViewport::GetFullScreenMode(ngs::FullScreenMode *aFullScreenMode)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* readonly attribute float DevicePixelRatio; */
NS_IMETHODIMP
SDLViewport::GetDevicePixelRatio(float *aDevicePixelRatio)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* void SetVideoMode (in long videoWidth, in long videoHeight, in ngsFullScreenMode fullScreenMode,
 * in boolean useNativePixelRatio); */
NS_IMETHODIMP
SDLViewport::SetVideoMode(int32_t videoWidth, int32_t videoHeight,
                            ngs::FullScreenMode fullScreenMode, bool useNativePixelRatio)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* attribute boolean EnableTextInput; */
NS_IMETHODIMP
SDLViewport::GetEnableTextInput(bool *aEnableTextInput)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}
NS_IMETHODIMP
SDLViewport::SetEnableTextInput(bool aEnableTextInput)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}

/* attribute ngsBox2D TextInputRectangle; */
NS_IMETHODIMP
SDLViewport::GetTextInputRectangle(ngs::Box2D *aTextInputRectangle)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}
NS_IMETHODIMP
SDLViewport::SetTextInputRectangle(ngs::Box2D aTextInputRectangle)
{
    return NS_ERROR_NOT_IMPLEMENTED;
}
}