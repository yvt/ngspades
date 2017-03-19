#include <ITestInterface.h>
#include <iostream>
#include <codecvt>

#if defined(_MSC_VER)
#define NGS_API __declspec(dllexport)
#elif defined(__GNUC__)
#define NGS_API __attribute__((visibility("default")))
#else
#define NGS_API
#endif

namespace {
class TestClass : public ITestInterface
{
public:
    NS_DECL_THREADSAFE_ISUPPORTS
    NS_DECL_ITESTINTERFACE

    TestClass() {}

private:
    ~TestClass() {}
};

NS_IMPL_ISUPPORTS(TestClass, ITestInterface)

NS_IMETHODIMP
TestClass::Hello(const char16_t *str)
{
    std::cout << "Hello world!" << std::endl;
    std::cout << std::wstring_convert<std::codecvt_utf8_utf16<char16_t>, char16_t>{}
        .to_bytes(str) << std::endl;
    return NS_OK;
}
}

extern "C" NGS_API nsresult
NgsCreateTestInstance(ITestInterface **outInstance)
{
    (new TestClass())->QueryInterface(ITESTINTERFACE_IID, (void **)outInstance);
    return NS_OK;
}
