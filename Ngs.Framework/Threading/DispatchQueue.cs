//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
using System;
using System.Diagnostics;
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

        sealed class Work {
            public Action action;
            public ExecutionContext context;
            public ResultCell result;
        }

        sealed class ResultCell {
            public Exception exception;
        }

        /// <summary>
        /// Indicates that operation was successful.
        /// </summary>
        readonly static Exception SUCCESS_TAG = new Exception();

        internal DispatchQueue() {
            thread = new Thread(ThreadBody);
            thread.Start();
        }

        [DebuggerNonUserCode]
        private void ThreadBody() {
            while (running) {
                var work = queue.Take();
                if (work.result is ResultCell result) {
                    Exception outException = SUCCESS_TAG;
                    try {
                        InvokeWork(work);
                    } catch (Exception e) {
                        outException = e;
                    }
                    lock (result) {
                        result.exception = outException;
                        Monitor.Pulse(result);
                    }
                } else {
                    try {
                        InvokeWork(work);
                    } catch (Exception e) when (TryHandleException(e)) {
                    }
                }
            }
        }

        [DebuggerNonUserCode]
        private void InvokeWork(in Work work) {
            ExecutionContext.Run(work.context, callActionDelegate, work.action);
        }

        private readonly ContextCallback callActionDelegate = new ContextCallback(CallAction);
        [DebuggerNonUserCode]
        private static void CallAction(object state) {
            var action = (Action)state;
            action();
        }

        /// <summary>
        /// Occurs when an unhandled exception occurs in a delegate passed to
        /// <see cref="InvokeAsync" />.
        /// </summary>
        public event UnhandledExceptionEventHandler UnhandledException;

        private bool TryHandleException(Exception exception) {
            var e = new UnhandledExceptionEventArgs(exception, true);
            UnhandledException?.Invoke(this, e);
            return false;
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
        /// <remarks>
        /// An exception that occured while calling the delegate will be propagated back to the
        /// caller.
        /// </remarks>
        /// <exception name="System.Reflection.TargetInvocationException">
        /// An exception was thrown while calling the delegate.</exception>
        /// <param name="action">The delegate to be called.</param>
        [DebuggerNonUserCode]
        public void Invoke(Action action) {
            if (IsCurrent) {
                action();
                return;
            }

            var resultCell = new ResultCell();
            queue.Add(new Work()
            {
                action = action,
                context = ExecutionContext.Capture().CreateCopy(),
                result = resultCell,
            });

            lock (resultCell) {
                while (resultCell.exception == null) {
                    Monitor.Wait(resultCell);
                }

                if (resultCell.exception != SUCCESS_TAG) {
                    throw new System.Reflection.TargetInvocationException(resultCell.exception);
                }
            }
        }

        /// <summary>
        /// Calls a supplied delegate asynchronously on this queue.
        /// </summary>
        /// <remarks>
        /// An exception that occured while calling the delegate will result in an application
        /// termination. The <see cref="UnhandledException" /> event is raised before the
        /// application is terminated.
        /// </remarks>
        /// <param name="action">The delegate to be called.</param>
        [DebuggerNonUserCode]
        public void InvokeAsync(Action action) {
            queue.Add(new Work()
            {
                action = action,
                context = ExecutionContext.Capture().CreateCopy(),
            });
        }
    }
}