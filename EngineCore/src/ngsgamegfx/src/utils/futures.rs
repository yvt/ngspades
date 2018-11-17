//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use futures::{prelude::*, ready, stream::Peekable, task, Poll, Stream};
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use std::pin::Pin;

pub trait PrivateStreamExt: Stream {
    /// A stateful version of `StreamExt::map`.
    fn map_with_state<U, F, S>(self, init: S, f: F) -> MapWithState<Self, F, S>
    where
        Self: Sized,
        F: FnMut(Self::Item, &mut S) -> U,
    {
        MapWithState {
            stream: self,
            f_state: (f, init),
        }
    }

    /// Annotate each output item of a stream with boolean value indicating
    /// whether the item is the last output or not.
    ///
    /// Returns an `impl Stream<Item = (Self::Item, bool), Error = Self::Error>`.
    fn with_terminator(self) -> WithTerminator<Self>
    where
        Self: Sized,
    {
        WithTerminator {
            inner: self.peekable(),
            next: None,
        }
    }

    /*
    /// Similar to `futures::StreamExt::chain`, but accepts a function that
    /// produces the next stream instead.
    fn chain_with<T, F>(self, func: T) -> ChainWith<Self, T, F::Future, F::Item>
    where
        T: FnOnce(Self) -> F,
        F: IntoFuture<Error = Self::Error>,
        <F as IntoFuture>::Item: Stream<Item = Self::Item, Error = Self::Error>,
        Self: Sized,
    {
        ChainWith {
            state: ChainWithState::First(self, func),
        }
    }
    */
}

impl<T: ?Sized + Stream> PrivateStreamExt for T {}

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct MapWithState<S, F, T> {
    stream: S,
    f_state: (F, T),
}

impl<S, F, T> MapWithState<S, F, T> {
    unsafe_pinned!(stream: S);
    unsafe_unpinned!(f_state: (F, T));
}

impl<S, F, U, T> Stream for MapWithState<S, F, T>
where
    S: Stream,
    F: FnMut(S::Item, &mut T) -> U,
{
    type Item = U;

    fn poll_next(mut self: Pin<&mut Self>, cx: &task::LocalWaker) -> Poll<Option<U>> {
        let option = ready!(self.stream().poll_next(cx));
        let (f, state) = self.f_state();
        Poll::Ready(option.map(|x| f(x, state)))
    }
}

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct WithTerminator<T: Stream> {
    inner: Peekable<T>,
    next: Option<T::Item>,
}

impl<T: Stream> WithTerminator<T> {
    unsafe_pinned!(inner: Peekable<T>);
    unsafe_unpinned!(next: Option<T::Item>);
}

impl<T: Stream> Stream for WithTerminator<T> {
    type Item = (T::Item, bool);

    fn poll_next(mut self: Pin<&mut Self>, cx: &task::LocalWaker) -> Poll<Option<Self::Item>> {
        if self.next().is_none() {
            if let Some(x) = ready!(self.inner().poll_next(cx)) {
                *self.next() = Some(x);
            } else {
                return Poll::Ready(None);
            }
        }

        let is_last = ready!(self.inner().peek(cx)).is_none();

        Poll::Ready(Some((self.next().take().unwrap(), is_last)))
    }
}

/*
#[derive(Debug)]
#[allow(dead_code)]
#[must_use = "streams do nothing unless polled"]
pub struct ChainWith<S1, T, F, S2> {
    state: ChainWithState<S1, T, F, S2>,
}

#[derive(Debug)]
#[allow(dead_code)]
enum ChainWithState<S1, T, F, S2> {
    /// Emitting elements of the first stream
    First(S1, T),
    /// Waiting for the second stream
    Transfer(F, Option<S2>),
    /// Emitting elements of the second stream
    Second(S2),
    Temp,
}

impl<S, T, F> Stream for ChainWith<S, T, F::Future, F::Item>
where
    S: Stream,
    T: FnOnce(S) -> F,
    F: IntoFuture<Error = S::Error>,
    F::Item: Stream<Item = S::Item, Error = S::Error>,
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll_next(&mut self, cx: &mut task::Context) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            match self.state {
                ChainWithState::First(ref mut s1, _) => match s1.poll_next(cx) {
                    Ok(Async::Ready(None)) => (), // roll
                    x => return x,
                },
                ChainWithState::Transfer(ref mut f, ref mut s2_cell) => match f.poll(cx) {
                    Ok(Async::Ready(s2)) => {
                        *s2_cell = Some(s2);
                    }
                    Ok(Async::Pending) => return Ok(Async::Pending),
                    Err(x) => return Err(x),
                },
                ChainWithState::Second(ref mut s2) => return s2.poll_next(cx),
                ChainWithState::Temp => unreachable!(),
            }

            self.state = match replace(&mut self.state, ChainWithState::Temp) {
                ChainWithState::First(s1, t) => ChainWithState::Transfer(t(s1).into_future(), None),
                ChainWithState::Transfer(_, Some(s2)) => ChainWithState::Second(s2),
                _ => unreachable!(),
            };
        }
    }
}
*/
