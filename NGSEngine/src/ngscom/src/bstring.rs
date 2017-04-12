//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate libc;

use std::ops::{Deref, DerefMut};
use std::{str, mem, i32, fmt, slice, ptr};

#[doc(hidden)]
#[derive(Debug)]
#[repr(C)]
pub struct BStringHeader {
    pub vtable: *const BStringVtable,
    pub length: usize,
}

/// NgsCOM string type.
#[repr(C)]
pub struct BString {
    pub header: BStringHeader,
    pub raw_data: [u8; 0],
}

#[doc(hidden)]
#[derive(Debug)]
#[repr(C)]
pub struct BStringVtable {
    pub destruct: unsafe extern "C" fn(*mut BString),
}

unsafe extern "C" fn free_bstring(this: *mut BString) {
    libc::free(mem::transmute(this));
}

static BSTR_VTABLE: BStringVtable = BStringVtable {
    destruct: free_bstring,
};

impl BString {
    /// Creates a new instance of `BString` without initializing the contents.
    pub unsafe fn alloc_uninitialized(len: usize) -> *mut BString {
        assert!(len <= (i32::MAX / 2) as usize);

        let bstr_size = mem::size_of::<BStringHeader>() + len + 1;
        let bstr: *mut BString = mem::transmute(libc::malloc(bstr_size));
        assert!(bstr != ptr::null_mut()); // handle memory allocation failure

        (*bstr).header = BStringHeader{
            vtable: mem::transmute(&BSTR_VTABLE),
            length: len,
        };
        *(*bstr).data_mut().get_unchecked_mut(len) = 0; // null terminator
        bstr
    }

    /// Creates a new instance of `BString` and initializes it with the given `&str`.
    pub fn alloc(s: &str) -> *mut BString {
        unsafe {
            let bstr = BString::alloc_uninitialized(s.len());
            (*bstr).data_mut().clone_from_slice(s.as_bytes());
            bstr
        }
    }

    /// Extracts a slice of the underlying raw data.
    pub fn data(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(mem::transmute(&self.raw_data), self.len())}
    }

    /// Extracts a mutable slice of the underlying raw data.
    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(mem::transmute(&self.raw_data), self.len())}
    }

    /// Extracts a slice of the underlying string.
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.data()) }
    }

    /// Destroys a `BString` and frees the memory allocated for it.
    pub unsafe fn free(&mut self) {
        ((*self.header.vtable).destruct)(self as *mut BString);
    }

    /// Returns the length of the given `BString`, measured in bytes.
    pub fn len(&self) -> usize { self.header.length }
}

impl fmt::Display for BString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for BString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BString {{ header: {:?}, data: {:?} }}",
            self.header, self.as_str())
    }
}

/// A safe wrapper for `*mut BString`.
pub struct BStringRef(*mut BString);

impl BStringRef {
    /// Creates a new `BStringRef` whose contents initialized with the given `&str`.
    pub fn new(s: &str) -> BStringRef {
        BStringRef::from_raw(BString::alloc(s))
    }

    /// Creates a null `BStringRef`.
    pub fn null() -> BStringRef {
        BStringRef(ptr::null_mut())
    }

    /// Constructs a new `BStringRef` from a raw `*mut BString`.
    pub fn from_raw(raw: *mut BString) -> BStringRef {
        BStringRef(raw)
    }

    /// Consumes the `BStringRef`, returning the raw `*mut BString`.
    pub fn into_raw(self) -> *mut BString {
        self.0
    }

    /// Returns whether this `BStringRef` has a non-null reference to `BString`.
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Deref for BStringRef {
    type Target = BString;
    fn deref(&self) -> &BString {
        assert!(!self.is_null());
        unsafe { &*self.0 }
    }
}

impl DerefMut for BStringRef {
    fn deref_mut(&mut self) -> &mut BString {
        assert!(!self.is_null());
        unsafe { &mut *self.0 }
    }
}

impl Drop for BStringRef {
    fn drop(&mut self) {
        if !self.is_null() {
            unsafe { self.free() }
        }
    }
}

impl fmt::Display for BStringRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_null() {
            write!(f, "null")
        } else {
            write!(f, "{}", **self)
        }
    }
}

impl fmt::Debug for BStringRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_null() {
            write!(f, "BStringRef {{ null }}")
        } else {
            write!(f, "BStringRef {{ {:?} }}", **self)
        }
    }
}

#[test]
fn bstr_create() {
    assert_eq!(BStringRef::new("ladybugs awake").as_str(), "ladybugs awake");
}

#[test]
fn bstr_len() {
    assert_eq!(BStringRef::new("hoge").len(), 4);
}