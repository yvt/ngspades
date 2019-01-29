//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A cell whose current owner is represented by the possession of a token.
//!
//! The implementation here is dependent on the internals of
//! `tokenlock::{Token, TokenRef}`.
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::sync::atomic::Ordering;
use std::{fmt, ops};
use tokenlock::{Token, TokenRef};

/// A cell whose current owner is represented by the possession of a token.
#[derive(Debug)]
pub struct TokenCell<T> {
    owner: atom2::Atom<TokenRef>,
    cell: UnsafeCell<T>,
}

unsafe impl<T> Sync for TokenCell<T> {}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum TokenCellBorrowError {
    NotOwned,
}

impl<T> TokenCell<T> {
    pub fn new(x: T) -> Self {
        TokenCell {
            owner: atom2::Atom::empty(),
            cell: UnsafeCell::new(x),
        }
    }

    /// Acquire the ownership and borrow the contents.
    pub fn acquire<'a>(
        &'a self,
        new_claim: &'a mut Token,
    ) -> Result<TokenCellRef<'a, T>, TokenCellBorrowError> {
        use std::ptr::null;
        let token: TokenRef = (&*new_claim).into();
        match self
            .owner
            .compare_and_swap(&null(), Some(token), Ordering::Acquire)
        {
            Ok(_) => Ok(TokenCellRef {
                token_cell: self,
                _phantom: PhantomData,
            }),
            Err(_) => self.borrow(new_claim),
        }
    }

    /// Borrow the contents.
    pub fn borrow<'a>(
        &'a self,
        claim: &'a mut Token,
    ) -> Result<TokenCellRef<'a, T>, TokenCellBorrowError> {
        if self.owner.is_equal_to(claim, Ordering::Relaxed) {
            Ok(TokenCellRef {
                token_cell: self,
                _phantom: PhantomData,
            })
        } else {
            Err(TokenCellBorrowError::NotOwned)
        }
    }

    /// Relinquish the ownership.
    pub fn release(&self, claim: &mut Token) -> Result<(), TokenCellBorrowError> {
        match self.owner.compare_and_swap(claim, None, Ordering::Release) {
            Ok(_) => Ok(()),
            Err(_) => Err(TokenCellBorrowError::NotOwned),
        }
    }
}

pub struct TokenCellRef<'a, T: 'a> {
    token_cell: &'a TokenCell<T>,
    _phantom: PhantomData<&'a mut Token>,
}

impl<'a, T: 'a> TokenCellRef<'a, T> {
    /// Consume the lock guard and relinquish the ownership.
    pub fn release(this: Self) {
        this.token_cell.owner.store(None, Ordering::Release);
    }
}

impl<'a, T: 'a> ops::Deref for TokenCellRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.token_cell.cell.get() }
    }
}

impl<'a, T: 'a> ops::DerefMut for TokenCellRef<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.token_cell.cell.get() }
    }
}

impl<'a, T: 'a + fmt::Debug> fmt::Debug for TokenCellRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("TokenCellRef").field(&**self).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let cell = TokenCell::new(1);

        let mut owner1 = Token::new();
        let mut owner2 = Token::new();

        assert!(cell.borrow(&mut owner1).is_err());
        *cell.acquire(&mut owner1).unwrap() += 1;

        assert!(cell.acquire(&mut owner2).is_err());
        cell.borrow(&mut owner1).unwrap();
        cell.release(&mut owner1).unwrap();

        {
            let mut lock = cell.acquire(&mut owner2).unwrap();
            *lock += 2;
            assert_eq!(*lock, 4);

            assert!(cell.acquire(&mut owner1).is_err());

            TokenCellRef::release(lock);
        }

        assert!(cell.borrow(&mut owner2).is_err());
    }
}
