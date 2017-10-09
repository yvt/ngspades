//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::mem;
use std::sync::Mutex;
use ngscom::{BString, BStringRef, HResult, ComPtr, hresults, UnownedComPtr};
use ngsbase::{ITestInterface, ITestInterfaceTrait, ITestInterfaceVtbl};

com_impl! {
    #[derive(Debug)]
    class TestClass {
        com_private: TestClassPrivate;
        itestinterface: (ITestInterface, ITestInterfaceVtbl, TESTCLASS_VTABLE);
        stored_str: Mutex<String>,
    }
}

impl ITestInterfaceTrait for TestClass {
    fn get_hoge_attr(&self, retval: &mut BStringRef) -> HResult {
        *retval = {
            let lock = self.stored_str.lock().unwrap();
            BStringRef::new(&format!("Stored str = {:?}", &*lock))
        };
        hresults::E_OK
    }
    fn set_hoge_attr(&self, value: Option<&BString>) -> HResult {
        println!("SetHogeAttr: I'm receiving this: {:?}", value.unwrap());
        if let Some(value) = value {
            *self.stored_str.lock().unwrap() = value.as_str().to_owned();
        }
        hresults::E_OK
    }
    fn hello(&self, value: Option<&BString>, retval: &mut BStringRef) -> HResult {
        println!("Hello! (got {:?})", value.unwrap());
        unsafe {
            println!("BString addr: {:x}, data: {:x}",
                     mem::transmute::<_, usize>(value),
                     mem::transmute::<_, usize>(&value.unwrap().data()[0]));
        }
        *retval = BStringRef::new("hOI! \0(null character here)");
        println!("Returning {:?}", retval);
        hresults::E_OK
    }
    fn simple_method(&self) -> HResult {
        hresults::E_OK
    }
    fn do_callback(&self, target: UnownedComPtr<ITestInterface>) -> HResult {
        if target.is_null() {
            return hresults::E_POINTER;
        }
        let mut hello_ret = BStringRef::null();
        target.hello(Some(&BStringRef::new("hello from do_callback")), &mut hello_ret).unwrap();
        println!("do_callback -> {:?}", target.do_callback(UnownedComPtr::null()));
        hresults::E_OK
    }
}

impl TestClass {
    fn new() -> ComPtr<ITestInterface> {
        ComPtr::from(&TestClass::alloc(TestClass {
            com_private: Self::new_private(),
            stored_str: Mutex::new("stored_str is not set yet!".to_owned()),
        }).0)
    }
}

#[no_mangle]
pub unsafe extern "C" fn create_test_instance(retval: &mut ComPtr<ITestInterface>) -> HResult {
    *retval = TestClass::new();
    hresults::E_OK
}
