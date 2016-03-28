//! Promises in Rust.
//! See the `Promise` struct for more details.

#![warn(missing_docs)]

#[cfg(test)]
mod tests;

use std::thread;
use std::sync::mpsc::channel;
//use std::thread::JoinHandle;
use std::marker::{Send};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};

/// A promise is a way of doing work in the background. The promises in
/// this library have the same featureset as those in Ecmascript 5.
///
/// # Promises
/// Promises (sometimes known as "futures") are objects that represent
/// asynchronous tasks being run in the background, or results which
/// will exist in the future.
/// A promise will be in state of running, fulfilled, rejected, or done.
/// In order to use the results of a fulfilled promise, one attaches another
/// promise to it (i.e. via `then`). Like their Javascript counterparts,
/// promises can return an error (of type `E`). Unlike Javascript, success or /// failure is indicated by a `Result` type. This is much more Rustic and
/// allows existing functions which return a `Result<T, E>` to be used.
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
    pub fn then<T2, E2, F1, F2>(self, callback: F1, errback: F2)
                                -> Promise<T2, E2>
    where T2: Send + 'static, E2: Send + 'static,
    F1: FnOnce(T) -> Result<T2, E2>, F2: FnOnce(E) -> Result<T2, E2>,
    F1: Send + 'static, F2: Send + 'static {
        let recv = self.receiver;
        let (tx, rx) = channel();

        thread::spawn(move || {
            Promise::impl_then(tx, recv, callback, errback);
        });

        Promise { receiver: rx }
    }

    /// Chains a function to be called after this promise resolves,
    /// using a `Result` type.
    pub fn then_result<T2, E2, F>(self, callback: F) -> Promise<T2, E2>
    where T2: Send + 'static, E2: Send + 'static,
    F: FnOnce(Result<T, E>) -> Result<T2, E2>, F: Send + 'static {
        let recv = self.receiver;
        let (tx, rx) = channel();

        thread::spawn(move || {
            Promise::impl_then_result(tx, recv, callback);
        });

        Promise { receiver: rx }
    }

    /// Creates a new promsie, which will eventually resolve to one of the
    /// values of the `Result<T, E>` type.
    pub fn new<F>(func: F) -> Promise<T, E>
    where F: FnOnce() -> Result<T, E>, F: Send + 'static {
        let (tx, rx) = channel();

        thread::spawn(move || {
            Promise::impl_new(tx, func);
        });

        Promise { receiver: rx }
    }

    /// Applies a promise to the first of some promises to become fulfilled.
    pub fn race(promises: Vec<Promise<T, E>>) -> Promise<T, E> {
        let recs = promises.into_iter().map(|p| p.receiver).collect();
        let (tx, rx) = channel::<Result<T, E>>();

        thread::spawn(move || {
            Promise::impl_race(tx, recs);
        });

        Promise { receiver: rx }
    }

    /// Calls a function with the result of all of the promises, or the error
    /// of the first promise to error.
    pub fn all(promises: Vec<Promise<T, E>>) -> Promise<Vec<T>, E> {
        let receivers: Vec<Receiver<Result<T, E>>> =
            promises.into_iter().map(|p| p.receiver).collect();
        let (tx, rx) = channel();

        thread::spawn(move || {
            Promise::impl_all(tx, receivers);
        });

        return Promise { receiver: rx };
    }

    /// Creates a promise that resolves to a value
    pub fn resolve(val: T) -> Promise<T, E> {
        Promise::from_result(Ok(val))
    }

    /// Creates a promise that resolves to an error.
    pub fn reject(val: E) -> Promise<T, E> {
        Promise::from_result(Err(val))
    }

    /// Creates a new promise that will resolve to the result value.
    pub fn from_result(result: Result<T, E>) -> Promise<T, E> {
        let (tx, rx) = channel();
        tx.send(result).unwrap();

        Promise { receiver: rx }
    }

    // Implementation Functions

    fn impl_new<F>(tx: Sender<Result<T, E>>, func: F)
    where F: FnOnce() -> Result<T, E>, F: Send + 'static {
        let result = func();
        tx.send(result).unwrap_or(());
    }

    fn impl_then<T2, E2, F1, F2>(tx: Sender<Result<T2, E2>>,
                                 rx: Receiver<Result<T, E>>,
                                 callback: F1, errback: F2)
    where T2: Send + 'static, E2: Send + 'static,
    F1: FnOnce(T) -> Result<T2, E2>, F2: FnOnce(E) -> Result<T2, E2>,
    F1: Send + 'static, F2: Send + 'static
    {
        if let Ok(message) = rx.recv() {
            match message {
                Ok(val) => tx.send(callback(val)).unwrap_or(()),
                Err(err) => tx.send(errback(err)).unwrap_or(())
            };
        }
    }

    fn impl_then_result<T2, E2, F>(tx: Sender<Result<T2, E2>>,
                                    rx: Receiver<Result<T, E>>,
                                    callback: F)
    where T2: Send + 'static, E2: Send + 'static,
    F: FnOnce(Result<T, E>) -> Result<T2, E2>, F: Send + 'static {

        if let Ok(result) = rx.recv() {
            tx.send(callback(result)).unwrap_or(());
        }
    }

    // Static methods

    fn impl_race(tx: Sender<Result<T, E>>,
                 mut recs: Vec<Receiver<Result<T, E>>>) {
        'outer: loop {
            // Don't get stuck in an infinite loop
            if recs.len() == 0 { return; }
            for i in 0..recs.len() {
                match recs[i].try_recv() {
                    Ok(val) => {
                        tx.send(val).unwrap_or(());
                        return;
                    }
                    Err(err) => {
                        if err == TryRecvError::Disconnected {
                            recs.remove(i);
                        }
                    }
                }
            }
        }
    }

    fn impl_all(tx: Sender<Result<Vec<T>, E>>,
                recs: Vec<Receiver<Result<T, E>>>) {
        let mut values: Vec<T> = Vec::with_capacity(recs.len());
        let mut mut_receivers = recs;
        'outer: loop {
            for i in 0..mut_receivers.len() {
                match mut_receivers[i].try_recv() {
                    Ok(val) => {
                        match val {
                            Ok(t) => values.push(t),
                            Err(e) => {
                                tx.send(Err(e)).unwrap_or(());
                                return;
                            }
                        }
                        mut_receivers.remove(i);
                    }
                    Err(err) => {
                        if err == TryRecvError::Disconnected {
                            mut_receivers.remove(i);
                        }
                    }
                }
            }
            // Check if we are finished waiting for promises
            // This can also happen if all promises panic
            if mut_receivers.len() == 0 {
                let result = Ok(values);
                tx.send(result).unwrap_or(());
                return; // Break from outer loop
            }
        }
    }
}
