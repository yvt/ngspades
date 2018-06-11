//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use send_cell::SendCell;
use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex, TryLockError};

use ngscom::{hresults, to_hresult, BStringRef, ComPtr, HResult, IAny, IUnknown, UnownedComPtr};

use core::{prelude::*, Context};
use hresults::{E_PF_LOCKED, E_PF_NOT_NODE, E_PF_THREAD};
use ngsbase::{INgsPFContext, INgsPFWorkspace, INgsPFWorkspaceListener, INgsPFWorkspaceTrait};
use nodes::{translate_context_error, INodeRef};
use viewport::{RootRef, Workspace, WorkspaceBuilder, WorkspaceError};
use ComContext;

fn translate_workspace_error(e: WorkspaceError) -> HResult {
    match e {
        WorkspaceError::OsError(_) => hresults::E_UNEXPECTED,
    }
}

com_impl! {
    class ComWorkspace {
        ingspf_workspace: INgsPFWorkspace;
        iany: IAny;
        @data: WorkspaceData;
    }
}

struct WorkspaceData {
    // We can't drop `Workspace` from an arbitrary thread. For now we just
    // forget its instance. This should be okay since in practice a workspace is
    // a singleton object. FIXME: Do something about this?
    workspace: ManuallyDrop<Mutex<SendCell<RefCell<Workspace>>>>,
    context: Arc<Context>,
    com_context: ComPtr<IAny>,
    root: RootRef,
}

impl ComWorkspace {
    pub fn new(
        listener: ComPtr<INgsPFWorkspaceListener>,
    ) -> Result<ComPtr<INgsPFWorkspace>, HResult> {
        // Retrieve the application info
        let mut app_name = BStringRef::new("");
        let mut app_ver_major = 0;
        let mut app_ver_minor = 0;
        let mut app_ver_revision = 0;

        listener
            .get_application_info(
                &mut app_name,
                &mut app_ver_major,
                &mut app_ver_minor,
                &mut app_ver_revision,
            )
            .into_result()
            .map_err(|x| x.into())?;

        let workspace = WorkspaceBuilder::new()
            .application_name(app_name.as_str())
            .application_version(
                app_ver_major as _,
                app_ver_minor as _,
                app_ver_revision as _,
            )
            .build()
            .map_err(translate_workspace_error)?;
        let context = workspace.context().clone();
        let com_context = (&ComContext::new(context.clone())).into();
        let root = workspace.root().clone();
        let workspace = Mutex::new(SendCell::new(RefCell::new(workspace)));
        Ok((&Self::alloc(WorkspaceData {
            workspace: ManuallyDrop::new(workspace),
            context,
            com_context,
            root,
        })).into())
    }

    fn com_context(&self) -> &ComContext {
        self.data.com_context.downcast_ref().unwrap()
    }
}

impl INgsPFWorkspaceTrait for ComWorkspace {
    fn get_context(&self, retval: &mut ComPtr<INgsPFContext>) -> HResult {
        *retval = (&self.data.com_context).into();
        hresults::E_OK
    }

    fn set_windows(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            let value = if value.is_null() {
                None
            } else {
                let inoderef = ComPtr::<INodeRef>::from(&*value);
                if inoderef.is_null() {
                    return Err(E_PF_NOT_NODE);
                }
                Some(inoderef.create_node_ref()?)
            };

            let mut lock = self.com_context().lock_producer_frame()?;
            let frame = lock.producer_frame_mut();
            self.data.root.windows().set(frame, value).unwrap();
            Ok(())
        })
    }

    fn start(&self) -> HResult {
        to_hresult(|| match self.data.workspace.try_lock() {
            Ok(guard) => {
                let cell = guard.try_get().ok_or(E_PF_THREAD)?;
                cell.borrow_mut()
                    .enter_main_loop()
                    .map_err(translate_workspace_error)?;
                Ok(())
            }
            Err(TryLockError::WouldBlock) => Err(E_PF_LOCKED),
            Err(_) => Err(hresults::E_UNEXPECTED),
        })
    }

    fn exit(&self) -> HResult {
        to_hresult(|| {
            let mut frame = self
                .data
                .context
                .lock_producer_frame()
                .map_err(translate_context_error)?;
            self.data.root.exit_loop(&mut frame).unwrap();
            Ok(())
        })
    }
}
