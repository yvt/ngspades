//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use RefEqArc;
use std::mem;
use std::cell::UnsafeCell;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Token(RefEqArc<()>);

impl Token {
    pub fn new() -> Self {
        Token(RefEqArc::new(()))
    }
}

impl !Sync for Token {}

/// A mutual exclusive primitive that can be accessed using a `Token`
/// with a very low over-head.
pub struct TokenLock<T: ?Sized> {
    keyhole: RefEqArc<()>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for TokenLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for TokenLock<T> {}

impl<T: ?Sized> fmt::Debug for TokenLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TokenLock")
            .field("keyhole", &self.keyhole)
            .finish()
    }
}

impl<T> TokenLock<T> {
    fn new(token: &Token, data: T) -> Self {
        Self {
            keyhole: token.0.clone(),
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> TokenLock<T> {
    #[inline]
    fn get_mut(&mut self) -> &mut T {
        unsafe { mem::transmute(self.data.get()) }
    }

    #[inline]
    fn read<'a: 'b, 'b>(&'a self, token: &'b Token) -> Option<&'b T> {
        if token.0 == self.keyhole {
            Some(unsafe { mem::transmute(self.data.get()) })
        } else {
            None
        }
    }

    #[inline]
    fn write<'a: 'b, 'b>(&'a self, token: &'b mut Token) -> Option<&'b mut T> {
        if token.0 == self.keyhole {
            Some(unsafe { mem::transmute(self.data.get()) })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test1() {
        let mut token = Token::new();
        let lock = TokenLock::new(&token, 1);
        assert_eq!(*lock.read(&token).unwrap(), 1);

        let guard = lock.write(&mut token).unwrap();
        assert_eq!(*guard, 1);

        // compilation should fail on the following line:
        // lock.read(&token)
    }
}
