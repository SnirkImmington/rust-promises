//! Contains functions wich are executed in promise threads.
//!
//! In order to assure that objects are being passed into promise
//! threads by value (and the threads take ownership of everything),
//! the threads themselves are just calls to functions. Only the
//! required objects are passed in.

use std::marker::Send;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};

use super::Promise;

// Promise constructor

pub fn new<T, E>(tx: Sender<Result<T, E>>,
                 func: Box<FnOnce() -> Result<T, E>>)
where T: Send + 'static, E: Send + 'static {
    let result = func();
    tx.send(result).unwrap_or(());
}

// Promise singletons

pub fn then<T, E, T2, E2>( tx: Sender<Result<T2, E2>>,
                           rx: Receiver<Result<T, E>>,
                      callback: fn(t: T) -> Result<T2, E2>,
                      errback: fn(e: E) -> Result<T2, E2>)
    where T: Send + 'static, E: Send + 'static,
         T2: Send + 'static, E2: Send + 'static {

    if let Ok(message) = rx.recv() {
        // Promise returned successfully
        match message {
            Ok(val) => tx.send(callback(val)).unwrap_or(()),
            Err(err) => tx.send(errback(err)).unwrap_or(())
        };
    }
    else { // Unable to receive, last promise panicked
        // Do nothing
    }
}

pub fn then_result<T, E, T2, E2>(rx: Receiver<Result<T, E>>,
                                 tx: Sender<Result<T2, E2>>,
                                 callback: fn(Result<T, E>) -> Result<T2, E2>)
    where T: Send + 'static, E: Send + 'static,
T2: Send + 'static, E2: Send + 'static {

    if let Ok(result) = rx.recv() {
        tx.send(callback(result)).unwrap_or(());
    }
    else { } // Receive failed, last promise panicked
}


