#include <ITestInterface.h>
#include <codecvt>
#include <iostream>

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

/* bstring Hello (in bstring str); */
NS_IMETHODIMP
TestClass::Hello(const ngs::BString *str, ngs::BString **_retval)
{
    std::cout << "Hello world!" << std::endl;
    std::cout << std::wstring_convert<std::codecvt_utf8_utf16<char16_t>, char16_t>{}.to_bytes(*str)
              << std::endl;
    *_retval = ngs::BString::Create<>(u"hOI! \0(null character here)").release();
    return NS_OK;
}

/* attribute bstring HogeAttr; */
NS_IMETHODIMP
TestClass::GetHogeAttr(ngs::BString **aHogeAttr)
{
    *aHogeAttr = ngs::BString::Create<>(u"You successfully GetHogeAttr'd!").release();
    return NS_OK;
}
NS_IMETHODIMP
TestClass::SetHogeAttr(const ngs::BString *aHogeAttr)
{
    std::cout << "SetHogeAttr: I'm getting this: ";
    std::cout << std::wstring_convert<std::codecvt_utf8_utf16<char16_t>, char16_t>{}.to_bytes(
                   *aHogeAttr)
              << std::endl;
    return NS_OK;
}
}

extern "C" NGS_API nsresult
NgsCreateTestInstance(ITestInterface **outInstance)
{
    (new TestClass())->QueryInterface(ITESTINTERFACE_IID, (void **)outInstance);
    return NS_OK;
}
