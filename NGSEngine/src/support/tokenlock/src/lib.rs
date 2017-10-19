//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Cell type whose contents can be accessed only via an inforgeable token.
//!
//! # Examples
//!
//! ```
//! # use tokenlock::*;
//! let mut token = Token::new();
//!
//! let lock = TokenLock::new(&token, 1);
//! assert_eq!(*lock.read(&token).unwrap(), 1);
//!
//! let mut guard = lock.write(&mut token).unwrap();
//! assert_eq!(*guard, 1);
//! *guard = 2;
//! ```
//!
//! The lifetime of the returned reference is limited by both of the `TokenLock`
//! and `Token`.
//!
//! ```compile_fail
//! # use tokenlock::*;
//! # use std::mem::drop;
//! # let mut token = Token::new();
//! # let lock = TokenLock::new(&token, 1);
//! # let guard = lock.write(&mut token).unwrap();
//! drop(lock); // compile error: cannot outlive `TokenLock`
//! ```
//!
//! ```compile_fail
//! # use tokenlock::*;
//! # use std::mem::drop;
//! # let mut token = Token::new();
//! # let lock = TokenLock::new(&token, 1);
//! # let guard = lock.write(&mut token).unwrap();
//! drop(token); // compile error: cannot outlive `Token`
//! ```
//!
//! This also prevents from forming a reference to the contained value when
//! there already is a mutable reference to it:
//!
//! ```compile_fail
//! # use tokenlock::*;
//! # let mut token = Token::new();
//! # let lock = TokenLock::new(&token, 1);
//! let write_guard = lock.write(&mut token).unwrap();
//! let read_guard = lock.read(&token).unwrap(); // compile error
//! ```
//!
//! While allowing multiple immutable references:
//!
//! ```
//! # use tokenlock::*;
//! # let mut token = Token::new();
//! # let lock = TokenLock::new(&token, 1);
//! let read_guard1 = lock.read(&token).unwrap();
//! let read_guard2 = lock.read(&token).unwrap();
//! ```
extern crate refeq;

use std::{mem, fmt};
use std::cell::UnsafeCell;
use refeq::RefEqArc;

/// An inforgeable token used to access the contents of a `TokenLock`.
///
/// This type is not `Clone` to ensure an exclusive access to `TokenLock`.
///
/// See the [module-level documentation] for more details.
///
/// [module-level documentation]: index.html
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Token(RefEqArc<()>);

unsafe impl Send for Token {}
unsafe impl Sync for Token {}

impl Token {
    pub fn new() -> Self {
        Token(RefEqArc::new(()))
    }
}

/// Token that cannot be used to access the contents of a `TokenLock`, but can
/// be used to create a new `TokenLock`.
///
/// See the [module-level documentation] for more details.
///
/// [module-level documentation]: index.html
///
/// # Examples
///
/// The parameter of `TokenLock::new` accepts `Into<TokenRef>`, so the following
/// codes are equivalent:
///
/// ```
/// # use tokenlock::*;
/// # let mut token = Token::new();
/// TokenLock::new(&token, 1);
/// TokenLock::new(TokenRef::from(&token), 1);
/// ```
///
/// `TokenRef` can be cloned while `Token` cannot:
///
/// ```
/// # use tokenlock::*;
/// let mut token = Token::new();
/// let token_ref = TokenRef::from(&token);
/// let lock1 = TokenLock::new(token_ref.clone(), 1);
/// let lock2 = TokenLock::new(token_ref.clone(), 2);
/// ```
///
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TokenRef(RefEqArc<()>);

impl<'a> From<&'a Token> for TokenRef {
    fn from(x: &'a Token) -> TokenRef {
        TokenRef(x.0.clone())
    }
}

/// A mutual exclusive primitive that can be accessed using a `Token`
/// with a very low over-head.
///
/// See the [module-level documentation] for more details.
///
/// [module-level documentation]: index.html
pub struct TokenLock<T: ?Sized> {
    keyhole: RefEqArc<()>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send + Sync> Send for TokenLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for TokenLock<T> {}

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
    pub fn read<'a>(&'a self, token: &'a Token) -> Option<&'a T> {
        if token.0 == self.keyhole {
            Some(unsafe { &*self.data.get() })
        } else {
            None
        }
    }

    #[inline]
    pub fn write<'a>(&'a self, token: &'a mut Token) -> Option<&'a mut T> {
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
    }
}
