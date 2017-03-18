#pragma once

#include <stdexcept>

#include <nsError.h>

namespace ngs {

class ComException final : public std::runtime_error
{
public:
    ComException(nsresult result);
    ComException(nsresult result, const std::string &message);
    ComException(nsresult result, const char *message);

    nsresult GetNSResult() const noexcept { return m_nsResult; }

private:
    nsresult m_nsResult;
};

template <class T>
inline nsresult
RunProtected(T functor)
{
    try {
        functor();
        return NS_OK;
    } catch (const ComException &ex) {
        return ex.GetNSResult();
    }
}
}
