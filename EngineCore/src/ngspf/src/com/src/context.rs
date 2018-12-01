//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use stickylock::{StickyMutex, StickyMutexGuard};
use tokenlock::{Token, TokenRef};

use ngsbase;
use ngscom::{com_impl, hresults, to_hresult, ComPtr, HResult, IAny, IUnknown};

use crate::hresults::{E_PF_LOCKED, E_PF_THREAD};
use crate::nodes::translate_context_error;
use crate::{nodes, INgsPFContext, INgsPFLayer, INgsPFNodeGroup, INgsPFWindow};
use ngspf_core::{Context, ProducerFrame};

com_impl! {
    #[derive(Debug)]
    class ComContext {
        ingspf_context: INgsPFContext;
        iany: IAny;
        @data: ContextData;
    }
}

#[derive(Debug)]
struct ContextData {
    context: Arc<Context>,
    producer_state: StickyMutex<ProducerState>,
    token_ref: TokenRef,
}

#[derive(Debug)]
struct ProducerState {
    stick_count: usize,

    /// `frame.is_some() == stick_count > 0`
    frame: Option<ProducerFrame>,

    /// `Token` can serve the same purpose as `ProducerFrame`, but is provided
    /// for convenience as we can't unlock two objects at the same time
    /// using a single `ProducerFrame`.
    token: Token,
}

/// An RAII lock guard for the producer frame state of `ComContext`.
///
/// It contains references to the following objects:
///
///  - `Token` that can be used to access the contained value of a `TokenLock`
///    created using a `TokenRef` returned by `ComContext::token_ref()`.
///  - `ProducerFrame` that can be used to access properties of NgsPF nodes
///    created for the associated `Context`.
///
#[derive(Debug)]
pub struct ProducerFrameLockGuard<'a>(StickyMutexGuard<'a, ProducerState>);

/// A reference to producer frame state.
#[derive(Debug)]
pub struct ProducerFrameRefMut<'a> {
    pub frame: &'a mut ProducerFrame,
    pub token: &'a mut Token,
}

impl ComContext {
    /// Construct a `ComContext` and get it as a `IUnknown`.
    pub fn new(context: Arc<Context>) -> ComPtr<IUnknown> {
        let token = Token::new();
        let token_ref = TokenRef::from(&token);
        let producer_state = ProducerState {
            stick_count: 0,
            frame: None,
            token,
        };
        Self::alloc(ContextData {
            context,
            producer_state: StickyMutex::new(producer_state),
            token_ref,
        })
    }

    /// Retrieve the underlying `Context`.
    pub fn context(&self) -> &Arc<Context> {
        &self.data.context
    }

    /// Get a `TokenRef`.
    ///
    /// Construct a `TokenLock` using the returned `TokenRef`. After you lock a
    /// producer frame using `lock_producer_frame`, you can access its contained
    /// value using a reference to `Token` obtained from a
    /// `ProducerFrameLockGuard`.
    pub fn token_ref(&self) -> &TokenRef {
        &self.data.token_ref
    }

    /// Attempt to lock a producer frame.
    pub fn lock_producer_frame(&self) -> Result<ProducerFrameLockGuard, HResult> {
        let mut producer_state = self.data.producer_state.try_lock().ok_or(E_PF_LOCKED)?;
        producer_state.stick(&self.data.context)?;
        Ok(ProducerFrameLockGuard(producer_state))
    }
}

impl ngsbase::INgsPFContextTrait for ComContext {
    fn create_node_group(&self, retval: &mut ComPtr<INgsPFNodeGroup>) -> HResult {
        *retval = nodes::ComNodeGroup::new((&self.as_com_ptr()).into());
        hresults::E_OK
    }

    fn create_window(&self, retval: &mut ComPtr<INgsPFWindow>) -> HResult {
        *retval = nodes::ComWindow::new((&self.as_com_ptr()).into());
        hresults::E_OK
    }

    fn create_layer(&self, retval: &mut ComPtr<INgsPFLayer>) -> HResult {
        *retval = nodes::ComLayer::new((&self.as_com_ptr()).into());
        hresults::E_OK
    }

    fn lock(&self) -> HResult {
        to_hresult(|| {
            let mut producer_state = self.data.producer_state.try_lock().ok_or(E_PF_LOCKED)?;
            producer_state.stick(&self.data.context)?;
            self.data.producer_state.stick();
            Ok(())
        })
    }

    fn unlock(&self) -> HResult {
        to_hresult(|| {
            let mut producer_state = self.data.producer_state.try_lock().ok_or(E_PF_THREAD)?;
            producer_state.unstick()?;
            self.data.producer_state.unstick().unwrap();
            Ok(())
        })
    }

    fn commit_frame(&self) -> HResult {
        self.data
            .context
            .commit()
            .map_err(translate_context_error)
            .err()
            .unwrap_or(hresults::E_OK)
    }
}

impl ProducerState {
    fn stick(&mut self, context: &Context) -> Result<(), HResult> {
        if self.stick_count == 0 {
            debug_assert!(self.frame.is_none());

            let frame = context
                .lock_producer_frame()
                .map_err(translate_context_error)?;

            self.frame = Some(frame);
        } else {
            debug_assert!(self.frame.is_some());
        }
        self.stick_count += 1;
        Ok(())
    }

    fn unstick(&mut self) -> Result<(), HResult> {
        debug_assert!(self.frame.is_some());
        if self.stick_count == 0 {
            return Err(E_PF_THREAD);
        }
        self.stick_count -= 1;
        if self.stick_count == 0 {
            self.frame = None;
        }
        Ok(())
    }
}

impl<'a> ProducerFrameLockGuard<'a> {
    /// Get a reference to the `ProducerFrame`.
    pub fn producer_frame(&self) -> &ProducerFrame {
        self.0.frame.as_ref().unwrap()
    }

    /// Get a mutable reference to the `ProducerFrame`.
    ///
    /// Use `get_mut` if you need the references of `ProducerFrame` and `Token`
    /// at the same time.
    pub fn producer_frame_mut(&mut self) -> &mut ProducerFrame {
        self.0.frame.as_mut().unwrap()
    }

    /// Get a reference to the `Token`.
    pub fn token(&self) -> &Token {
        &self.0.token
    }

    /// Get a mutable reference to the `Token`.
    ///
    /// Use `get_mut` if you need the references of `ProducerFrame` and `Token`
    /// at the same time.
    pub fn token_mut(&mut self) -> &mut Token {
        &mut self.0.token
    }

    /// Get a `ProducerFrameRefMut`, containing mutable references to
    /// both of `ProducerFrame` and `Token`.
    pub fn get_mut(&mut self) -> ProducerFrameRefMut {
        let ref mut state = *self.0;
        ProducerFrameRefMut {
            frame: state.frame.as_mut().unwrap(),
            token: &mut state.token,
        }
    }
}

impl<'a> Drop for ProducerFrameLockGuard<'a> {
    fn drop(&mut self) {
        self.0.unstick().unwrap();
    }
}
