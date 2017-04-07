#[macro_use]
extern crate ngscom;

#[macro_use]
extern crate lazy_static;

use std::os::raw::c_void;
use std::mem;
use ngscom::{IUnknown, IUnknownTrait, BString, BStringRef, HResult, IID, E_OK, ComPtr};

iid!(IID_ITESTINTERFACE =
    0x35edff15, 0x0b38, 0x47d8, 0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d);

com_interface! {
    interface (ITestInterface, ITestInterfaceTrait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE,
        vtable: ITestInterfaceVTable,
        thunk: ITestInterfaceThunk,

        fn get_hoge_attr(retval: &mut BStringRef) -> HResult;
        fn set_hoge_attr(value: &BString) -> HResult;
        fn hello(value: &BString, retval: &mut BStringRef) -> HResult;
        fn simple_method() -> HResult;
    }
}

com_impl!(TESTCLASS_VTABLE, ITestInterfaceVTable, ITestInterface, TestClass);

#[derive(Debug)]
struct TestClass {
    itestinterface: ITestInterface,
}

impl IUnknownTrait for TestClass {
    unsafe fn query_interface(&mut self, iid: &IID, object: *mut *mut c_void) -> HResult {
        println!("query_interface({:?}, {}): not implemented", self, iid);
        self.add_ref();
        *object = mem::transmute(self);
        E_OK
        // TODO: implement somehow!
    }
    unsafe fn add_ref(&mut self) -> u32 {
        println!("add_ref({:?}): not implemented", self);
        0
        // TODO: implement somehow!
    }
    unsafe fn release(&mut self) -> u32 {
        println!("release({:?}): not implemented", self);
        0
        // TODO: implement somehow!
    }
}

impl ITestInterfaceTrait for TestClass {
    fn get_hoge_attr(&mut self, retval: &mut BStringRef) -> HResult {
        *retval = BStringRef::new("You successfully GetHogeAttr'd!");
        E_OK
    }
    fn set_hoge_attr(&mut self, value: &BString) -> HResult {
        println!("SetHogeAttr: I'm getting this: {:?}", value);
        E_OK
    }
    fn hello(&mut self, value: &BString, retval: &mut BStringRef) -> HResult {
        println!("Hello! (got {:?})", value);
        unsafe { println!("BString addr: {:x}, data: {:x}",
            mem::transmute::<_, usize>(value), mem::transmute::<_, usize>(&value.data()[0])) };
        *retval = BStringRef::new("hOI! \0(null character here)");
        E_OK
    }
    fn simple_method(&mut self) -> HResult {
        E_OK
    }
}

impl TestClass {
    fn new() -> *mut TestClass {
        let new_box = Box::new(TestClass {
            itestinterface: ITestInterface {
                vtable: &*TESTCLASS_VTABLE as *const ITestInterfaceVTable
            }
        });
        Box::into_raw(new_box)
    }
}

#[no_mangle]
pub unsafe extern fn create_test_instance(retval: *mut *mut ITestInterface) -> HResult {
    *retval = &mut (*TestClass::new()).itestinterface;
    E_OK
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
