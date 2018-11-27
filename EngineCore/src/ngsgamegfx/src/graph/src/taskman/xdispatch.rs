//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use xdispatch::Queue;

use super::Executor;

impl<'a> Executor for Queue {
    fn spawn(&self, f: impl FnOnce(&Self) + Send + 'static) {
        let queue = self.clone();
        self.r#async(move || f(&queue));
    }
}
