//! Promises in Rust.
//! See the `Promise` struct for more details.

#![warn(missing_docs)]

use std::thread;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::thread::JoinHandle;
use std::marker::{Send};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};

/// A promise is a way of doing work in the background. The promises in
/// this library have the same featureset as those in Ecmascript 5.
///
/// # Promises
/// Promises (sometimes known as "futures") are objects that represent
/// asynchronous tasks being run in the background, or results which
/// will exist in the future.
/// A promise will be in state of running, fulfilled, or done. In order to
/// use the results of a fulfilled promise, one attaches another promise
/// to it (i.e. via `then`). Like their Javascript counterparts, promises can
/// return an error (of type `E`).
///
/// # Panics
/// If the function being executed by a promise panics, it does so silently.
/// The panic will not resurface in the thread which created the promise,
/// and promises waiting on its result will never be called. In addition,
/// the `all` and `race` proimse methods will _ignore_ "dead" promises. They
/// will remove promises from their lists, and if there aren't any left
/// they will silently exit without doing anything.
///
/// Unfortunately, panics must be ignored for two reasons:
/// * Panic messages don't have a concrete type yet in Rust. If they did,
/// promiess would be able to inspect their predecessors' errors.
/// * Although a `Receiver` can correctly handle its paired `Sender` being
/// dropped, such as during a panic, for reasons stated above the "message"
/// of the panic is not relayed.
///
/// Finally, Ecmascript promises themselves do have the ability to return
/// and error type, represented as a `Result<T, E>` here. Thus, one should
/// use `try!` and other error handling rather than calls to `unwrap()`.
pub struct Promise<T: Send, E: Send> {
    receiver: Receiver<Result<T, E>>
}

impl<T: Send + 'static, E: Send + 'static> Promise<T, E> {

    /// Chains a function to be called after this promise resolves.
    pub fn then<T2, E2>(self, callback: fn(f: T) -> Result<T2, E2>,
                              errback:  fn(e: E) -> Result<T2, E2>)
                        -> Promise<T2, E2>
    where T2: Send + 'static, E2: Send + 'static {
        let recv = self.receiver;
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            Promise::promise_then(tx, recv, callback, errback);
        });
        recv.recv();
        return Promise { receiver: rx };
    }

    /// Chains a function to be called after this promise resolves,
    /// using a `Result` type.
    pub fn then_result<T2, E2>(self,
                               callback: fn(Result<T, E>) -> Result<T2, E2>)
                               -> Promise<T2, E2>
    where T2: Send + 'static, E2: Send + 'static {
        let recv = self.receiver;
        let recv2 = self.receiver;
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            Promise::promise_then_result(tx, callback);
        });
        return Promise { receiver: rx };
    }

    /// Creates a new promsie
    pub fn new<F>(func: F) -> Promise<T, E>
        where F: Send + 'static + FnOnce() -> Result<T, E> {
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            let result = func();
            let _ = tx.send(result).unwrap_or(());
        });

        return Promise { receiver: rx };
    }

    /// Applies a promise to the first of some promises to become fulfilled.
    pub fn race(promises: Vec<Promise<T, E>>) -> Promise<T, E> {
        let mut receivers = promises.into_iter().map(|p| p.receiver).collect();
        let (tx, rx) = channel();

        thread::spawn(move || {
            Promise::race_function(receivers, tx);
        });
        return Promise { receiver: rx };
    }

    /// Calls a function with the result of all of the promises, or the error
    /// of the first promise to error.
    pub fn all(promises: Vec<Promise<T, E>>) -> Promise<Vec<T>, E> {
        let receivers: Vec<Receiver<Result<T, E>>> =
            promises.into_iter().map(|p| p.receiver).collect();
        let (tx, rx) = channel();

        thread::spawn(move || {
            Promise::all_function(receivers, tx);
        });

        return Promise { receiver: rx };
    }

    /// Creates a promise that resolves to a value
    pub fn resolve(val: T) -> Promise<T, E> {
    }

    pub fn reject(val: E) -> Promise<T, E> {
    }

    // Implementation Functions

    fn promise_new(tx: Sender<Result<T, E>>,
                   func: Box<FnOnce() -> Reuslt<T, E>>) {
        let result = func();
        tx.send(result).unwrap_or(());
    }

    fn promise_then<T2, E2>(tx: Sender<Result<T2, E2>>,
                            rx: Receiver<Result<T, E>>,
                            callback: fn(T) -> Result<T2, E2>,
                            errback: fn(E) -> Result<T2, E2>)
    where T2: Send + 'static, E2: Send + 'static {
        if let Ok(message) = rx.recv() {
            match message {
                Ok(val) => tx.send(callback(val)).unwrap_or(()),
                Err(err) => tx.send(errback(err)).unwrap_or(())
            };
        }
    }

    fn promise_then_result<T2, E2>(tx: Sender<Result<T, E>>,
                                   rx: Receiver<Result<T2, E2>>,
                                 callback: fn(Result<T, E>) -> Result<T2, E2>)
    where T2: Send + 'static, E2: Send + 'static {

        if let Ok(result) = rx.recv() {
            tx.send(callback(result)).unwrap_or(());
        }
    }

    // Static methods

    fn promise_race(receivers: Vec<Receiver<Result<T, E>>>,
                           tx: Sender<Result<T, E>>) {
        'outer: loop {
            for i in 0..receivers.len() {
                match receivers[i].try_recv() {
                    Ok(val) => {
                        let _ = tx.send(val).unwrap_or(());
                        return;
                    }
                    Err(err) => {
                        if err == TryRecvError::Disconnected {
                            receivers.remove(i);
                        }
                    }
                }
            }
        }
    }

    fn promise_all(receivers: Vec<Receiver<Result<T, E>>>,
                          tx: Sender<Result<Vec<T>, E>>) {
        let mut values: Vec<T> = Vec::with_capacity(receivers.len());
        let mut mut_receivers = receivers;
        'outer: loop {
            for rec in receivers {
                match rec.try_recv() {
                    Ok(val) => {
                        match val {
                            Ok(t) => values.push(t),
                            Err(e) => {
                                let _ = tx.send(Err(e)).unwrap_or(());
                                return;
                            }
                        }
                    }
                    Err(err) => {
                        // Remove dead promises?
                    }
                }
            }
        }
    }
}
