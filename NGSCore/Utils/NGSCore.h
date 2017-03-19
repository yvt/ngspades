#pragma once

#include <BString.h>
#include <mozilla/RefPtr.h>
#include <RefCounted.h>

// namespace unification
namespace ngs
{
template <class T>
using RefPtr = ::RefPtr<T>;
}
