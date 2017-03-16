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

    nsresult GetNSResult();

private:
    nsresult m_nsResult;
};
}
