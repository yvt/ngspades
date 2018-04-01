//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[cfg(not(target_os = "macos"))]
mod os {
    pub fn autorelease_pool_scope<T, S>(cb: T) -> S
    where
        T: FnOnce(&mut AutoreleasePool) -> S,
    {
        cb(&mut AutoreleasePool)
    }

    pub struct AutoreleasePool;

    impl AutoreleasePool {
        pub fn drain(&mut self) {
            // No-op
        }
    }
}

#[cfg(target_os = "macos")]
mod os {
    use zangfx::backends::metal::metal;
    use metalutils::OCPtr;

    pub fn autorelease_pool_scope<T, S>(cb: T) -> S
    where
        T: FnOnce(&mut AutoreleasePool) -> S,
    {
        let mut op = AutoreleasePool(Some(unsafe {
            OCPtr::from_raw(metal::NSAutoreleasePool::alloc().init()).unwrap()
        }));
        cb(&mut op)
    }

    pub struct AutoreleasePool(Option<OCPtr<metal::NSAutoreleasePool>>);

    impl AutoreleasePool {
        pub fn drain(&mut self) {
            self.0 = None;
            self.0 =
                Some(unsafe { OCPtr::from_raw(metal::NSAutoreleasePool::alloc().init()).unwrap() });
        }
    }
}

pub use self::os::*;
