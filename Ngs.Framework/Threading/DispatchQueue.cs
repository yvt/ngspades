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
using System.ComponentModel;

namespace Ngs.Threading {
    /// <summary>
    /// Manages a work queue for a specific thread.
    /// </summary>
    /// <remarks>
    /// The design of this class is yet to be worked on.
    /// </remarks>
    public sealed class DispatchQueue : ISynchronizeInvoke {
        Thread thread;
        BlockingCollection<Work> queue = new BlockingCollection<Work>();
        bool running = true;

        sealed class Work : IAsyncResult {
            public Action action;
            public ExecutionContext context;
            public ResultCell result;

            // `IAsyncResult` implementation (Only used with `BeginInvoke` and `EndInvoke`)
            object IAsyncResult.AsyncState => null;
            WaitHandle IAsyncResult.AsyncWaitHandle => throw new NotImplementedException();
            bool IAsyncResult.CompletedSynchronously => false;
            bool IAsyncResult.IsCompleted => result.exception != null;

            public void Wait() {
                var resultCell = this.result;
                lock (resultCell) {
                    while (resultCell.exception == null) {
                        Monitor.Wait(resultCell);
                    }

                    if (resultCell.exception != SUCCESS_TAG && resultCell.exception != SUCCESS_SYNC_TAG) {
                        throw new System.Reflection.TargetInvocationException(resultCell.exception);
                    }
                }
            }
        }

        sealed class ResultCell {
            public Exception exception;
        }

        /// <summary>
        /// Indicates that operation was successful.
        /// </summary>
        readonly static Exception SUCCESS_TAG = new Exception();

        /// <summary>
        /// Indicates that operation was completed synchronously.
        /// </summary>
        readonly static Exception SUCCESS_SYNC_TAG = new Exception();

        internal DispatchQueue() {
            thread = new Thread(ThreadBody);
            thread.Name = "Ngs.UI dispatch queue";
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

            var work = new Work()
            {
                action = action,
                context = ExecutionContext.Capture().CreateCopy(),
                result = new ResultCell(),
            };
            queue.Add(work);

            work.Wait();
        }

        /// <summary>
        /// Calls a supplied delegate asynchronously on this queue.
        /// </summary>
        /// <remarks>
        /// An exception that occured while calling the delegate is rethrown when
        /// <see cref="EndInvoke" /> is called.
        /// </remarks>
        /// <param name="action">The delegate to be called.</param>
        /// <returns>An <see cref="IAsyncResult" /> that represents the result of the operation.
        /// </returns>
        [DebuggerNonUserCode]
        public IAsyncResult BeginInvoke(Action action) {
            var resultCell = new ResultCell();
            var work = new Work()
            {
                result = resultCell,
            };

            if (IsCurrent) {
                // The operation can be completed synchronously
                try {
                    action();
                    resultCell.exception = SUCCESS_SYNC_TAG;
                } catch (Exception e) {
                    resultCell.exception = e;
                }
                return work;
            }

            work.action = action;
            work.context = ExecutionContext.Capture().CreateCopy();

            queue.Add(work);
            return work;
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

        #region ISynchronizeInvoke implementation

        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.InvokeRequired" />.
        /// </summary>
        public bool InvokeRequired => !IsCurrent;

        // A call `EndInvoke` corresponds to one of the `BeginInvoke` overloads. In cases where it's
        // `BeginInvoke(Delete, object[])`, we must use a different concrete type of `IAsyncResult`
        // so we can return the value returned by a supplied delegate.
        sealed class DelegateWork : IAsyncResult {
            public Work work;

            public object result;

            object IAsyncResult.AsyncState => null;
            WaitHandle IAsyncResult.AsyncWaitHandle => ((IAsyncResult)work).AsyncWaitHandle;
            bool IAsyncResult.CompletedSynchronously => ((IAsyncResult)work).CompletedSynchronously;
            bool IAsyncResult.IsCompleted => ((IAsyncResult)work).IsCompleted;
        }

        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.BeginInvoke(Delegate, object[])" />.
        /// </summary>
        public IAsyncResult BeginInvoke(Delegate method, object[] args) {
            var delegateWork = new DelegateWork();
            delegateWork.work = (Work)BeginInvoke(() => {
                delegateWork.result = method.DynamicInvoke(args);
            });
            return delegateWork;
        }

        /// <summary>
        /// Implemenets <see cref="ISynchronizeInvoke.EndInvoke(IAsyncResult)" />.
        /// </summary>
        public object EndInvoke(IAsyncResult result) {
            switch (result) {
                case Work work:
                    work.Wait();
                    return null;
                case DelegateWork work:
                    work.work.Wait();
                    return work.result;
                default:
                    throw new ArgumentException(nameof(result));
            }
        }

        /// <summary>
        /// Implements <see cref="ISynchronizeInvoke.Invoke(Delegate, object[])" />.
        /// </summary>
        public object Invoke(Delegate method, object[] args) {
            object resultCell = null;
            Invoke(() => resultCell = method.DynamicInvoke(args));
            return resultCell;
        }

        #endregion
    }
}