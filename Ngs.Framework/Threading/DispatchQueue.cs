//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Security;
using System.Threading;
using System.Collections.Concurrent;

namespace Ngs.Threading {
    /// <summary>
    /// Manages a work queue for a specific thread.
    /// </summary>
    /// <remarks>
    /// The design of this class is yet to be worked on.
    /// </remarks>
    public sealed class DispatchQueue {
        Thread thread;
        BlockingCollection<Work> queue = new BlockingCollection<Work>();
        bool running = true;

        struct Work {
            public Action action;
            public ExecutionContext context;
            public ManualResetEventSlim doneEvent;
        }

        internal DispatchQueue() {
            thread = new Thread(ThreadBody);
            thread.Start();
        }

        private void ThreadBody() {
            while (running) {
                var work = queue.Take();
                ExecutionContext.Run(work.context, callActionDelegate, work.action);
                if (work.doneEvent != null) {
                    work.doneEvent.Set();
                }
            }
        }

        private readonly ContextCallback callActionDelegate = new ContextCallback(CallAction);
        private static void CallAction(object state) {
            var action = (Action)state;
            action();
        }

        /// <summary>
        /// Indicates whether the current execution context is bound to this queue.
        /// </summary>
        /// <returns><c>true</c> if the code is running within the context of this queue;
        /// otherwise, <c>false</c>.</returns>
        public bool IsCurrent {
            get => thread == Thread.CurrentThread;
        }

        internal void Exit() {
            running = false;
            InvokeAsync(() => { });
        }

        /// <summary>
        /// Throws an exeception if the currently running thread is not bound to this queue.
        /// </summary>
        /// <exception name="InvalidOperationException">The current thread is not bound to this
        /// queue.</exception>
        public void VerifyAccess() {
            if (!IsCurrent) {
                throw new InvalidOperationException("This operation is not permitted for the current thread.");
            }
        }

        /// <summary>
        /// Calls a supplied delegate synchronously on this queue.
        /// </summary>
        /// <param name="action">The delegate to be called.</param>
        public void Invoke(Action action) {
            if (IsCurrent) {
                action();
                return;
            }

            var doneEvent = new ManualResetEventSlim();
            queue.Add(new Work()
            {
                action = action,
                context = ExecutionContext.Capture().CreateCopy(),
                doneEvent = doneEvent,
            });
            doneEvent.Wait();
        }

        /// <summary>
        /// Calls a supplied delegate asynchronously on this queue.
        /// </summary>
        /// <param name="action">The delegate to be called.</param>
        public void InvokeAsync(Action action) {
            queue.Add(new Work()
            {
                action = action,
                context = ExecutionContext.Capture().CreateCopy(),
            });
        }
    }
}