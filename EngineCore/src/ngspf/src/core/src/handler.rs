//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
pub struct CommitHandlerList(Vec<Box<FnMut() + Send + 'static>>);

impl CommitHandlerList {
    pub fn new() -> Self {
        CommitHandlerList(Vec::new())
    }

    pub fn emit(&mut self) {
        for x in self.0.iter_mut() {
            x();
        }
    }

    pub fn push<F: FnMut() + Send + 'static>(&mut self, handler: F) {
        self.0.push(Box::new(handler));
    }
}

impl std::fmt::Debug for CommitHandlerList {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("CommitHandlerList").finish()
    }
}
