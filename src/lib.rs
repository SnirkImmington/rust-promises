//! Promises in Rust.

#![warn(missing_docs)]

use std::thread;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::thread::JoinHandle;
use std::marker::{Send};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};

mod internal;
use internal::*;

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

impl<T, E> Promise<T, E> where T: Send + 'static, E: Send + 'static {

    /// Chains a function to be called after this promise resolves.
    pub fn then<T2, E2>(self, callback: fn(f: T) -> Result<T2, E2>,
                              errback:  fn(e: E) -> Result<T2, E2>)
                        -> Promise<T2, E2>
    where T2: Send + 'static, E2: Send + 'static {
        let recv = self.receiver;
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            internal::then(tx, recv, callback, errback);
        });
        return Promise { receiver: rx };
    }

    /// Chains a function to be called after this promise resolves.
    pub fn then_result<T2, E2>(self,
                               callback: fn(input: Result<T, E>) -> Result<T2, E2>)
                               -> Promise<T2, E2>
    where T2: Send + 'static, E2: Send + 'static {
        let recv = self.receiver;
        let recv2 = self.receiver;
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            if let Ok(result) = recv.recv() { // Blocking receive until message
                let _ = tx.send(callback(result)).unwrap_or(());
            }
            else { // Receive failed, origin was dropped
            }
        });
        return Promise { receiver: rx };
    }

    /// Creates a new promsie
    pub fn new<F>(func: F) -> Promise<T, E>
        where T: Send + 'static, E: Send + 'static,
              F: Send + 'static + FnOnce() -> Result<T, E> {
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            let result = func();
            let _ = tx.send(result).unwrap_or(());
        });

        return Promise { receiver: rx };
    }
}

pub fn race<T, E>(promises: Vec<Promise<T, E>>) -> Promise<T, E>
where T: Send + 'static, E: Send + 'static {
    let mut receivers = promises.into_iter().map(|p| p.receiver).collect();
    let (tx, rx) = channel();

    thread::spawn(move || {
        race_function(receivers, tx);
    });

    return Promise { receiver: rx };
}

pub fn all<T, E>(promises: Vec<Promise<T, E>>) -> Promise<Vec<T>, E>
where T: Send + 'static, E: Send + 'static {
    let receivers: Vec<Receiver<Result<T, E>>> = promises.into_iter().map(|p| p.receiver).collect();
    let (tx, rx) = channel();

    thread::spawn(move || {
        all_function(receivers, tx);
    });

    return Promise { receiver: rx };
}


pub fn resolve<T, E>(val: E) -> Promise<T, E> {
}

pub fn reject<T, E>(val: E) -> Promise<T, E> {
}

fn race_function<T, E>(receivers: Vec<Receiver<Result<T, E>>>, tx: Sender<Result<T, E>>)
where T: Send + 'static, E: Send + 'static {
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

fn all_function<T, E>(receivers: Vec<Receiver<Result<T, E>>>, tx: Sender<Result<Vec<T>, E>>)
where T: Send + 'static, E: Send + 'static {
    let mut values: Vec<T> = Vec::with_capacity(receivers.len());
    let mut mut_receivers = receivers; //.into_iter().map(|r| r.clone()).collect();
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
