#include <iostream>
#include <ITestInterface.h>

#if defined(_MSC_VER)
#define NGS_API __declspec(dllexport)
#elif defined(__GNUC__)
#define NGS_API __attribute__((visibility("default")))
#else
#define NGS_API
#endif

namespace
{
    class TestClass : public ITestInterface
    {
    public:
        NS_DECL_THREADSAFE_ISUPPORTS
        NS_DECL_ITESTINTERFACE

        TestClass()
        {
        }

    private:
        ~TestClass()
        {
        }
    };

    NS_IMPL_ISUPPORTS(TestClass, ITestInterface)

    NS_IMETHODIMP TestClass::Hello()
    {
        std::cout << "Hello world!" << std::endl;
        return NS_OK;
    }
}

extern "C" NGS_API nsresult NgsCreateTestInstance(ITestInterface **outInstance)
{
    (new TestClass())->QueryInterface(ITESTINTERFACE_IID, (void **)outInstance);
    return NS_OK;
}
