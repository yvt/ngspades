//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
// (based on `ngsgfx/src/backend/vulkan/command/tokenlock.rs`)
use refeq::RefEqArc;
use std::mem;
use std::cell::UnsafeCell;
use std::fmt;

/// An inforgeable token used to access the contents of a `TokenLock`.
///
/// This type is not `Clone` to ensure an exclusive access to `TokenLock`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Token(RefEqArc<()>);

impl Token {
    pub fn new() -> Self {
        Token(RefEqArc::new(()))
    }
}

/// A reference to `Token`. Cannot be used to access the contents of a
/// `TokenLock`, but can be used to create a new `TokenLock`.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TokenRef(RefEqArc<()>);

impl<'a> From<&'a Token> for TokenRef {
    fn from(x: &'a Token) -> TokenRef {
        TokenRef(x.0.clone())
    }
}

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
    pub fn new<S: Into<TokenRef>>(token: S, data: T) -> Self {
        Self {
            keyhole: token.into().0,
            data: UnsafeCell::new(data),
        }
    }
}

impl<T: ?Sized> TokenLock<T> {
    #[inline]
    #[allow(dead_code)]
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { mem::transmute(self.data.get()) }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn read<'a: 'b, 'b>(&'a self, token: &'b Token) -> Option<&'a T> {
        if token.0 == self.keyhole {
            Some(unsafe { &*self.data.get() })
        } else {
            None
        }
    }

    #[inline]
    pub fn write<'a: 'b, 'b>(&'a self, token: &'b mut Token) -> Option<&'a mut T> {
        if token.0 == self.keyhole {
            Some(unsafe { &mut *self.data.get() })
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
