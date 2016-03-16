//! Promises in Rust.

#![warn(incomplete_docs)]

use std::thread;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::thread::JoinHandle;
use std::marker::{Send};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};

/// A promise is a way of doing work in the background
pub struct Promise<T, E> where T: Send, E: Send {
    //sender: Sender<PromiseMessage<T, E>>, // Send messages to the promise
    receiver: Receiver<Result<T, E>> // Get the result of the promise for chaining (pass into then)
}

enum PromiseJoinType {
    Race,
    All
}

impl<T, E> Promise<T, E> where T: Send, E: Send {

    /// Chains a function to be called after this promise resolves.
    pub fn then<T2, E2>(self, callback: fn(f: T) -> Result<T2, E2>,
                              errback:  fn(e: E) -> Result<T2, E2>)
                        -> Promise<T2, E2>
    where T2: Send, E2: Send {
        let recv = self.receiver;
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            if let Ok(message) = recv.recv() { // Blocking receive until message
                match message {
                    Ok(val) => {
                        let _ = tx.send(callback(val)).unwrap_or(());
                    }
                    Err(err) => {
                        let _ = tx.send(errback(err)).unwrap_or(());
                    }
                }
            }
            else { // Receive failed, origin was dropped
            }
        });
        return Promise { receiver: rx };
    }

    /// Chains a function to be called after this promise resolves.
    pub fn then_result<T2, E2>(self,
                               callback: fn(input: Result<T, E>) -> Result<T2, E2>)
                               -> Promise<T2, E2>
    where T2: Sync, E2: Sync {
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
    pub fn new(func: fn() -> Result<T, E>) -> Promise<T, E> {
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            let result = func();
            let _ = tx.send(result).unwrap_or(());
        });

        return Promise { receiver: rx };
    }
}

pub fn race<T, E>(promises: Vec<Promise<T, E>>) -> Promise<T, E>
where T: Send, E: Send {
    let mut receivers = promises.into_iter().map(|x| x.receiver).collect();
    let (tx, rx) = channel();

    thread::spawn(move || {
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
    });
}

pub fn all<T, E>(promises: Vec<Promise<T, E>>) -> Promise<T, E> {
}


pub fn resolve<T, E>(val: E) -> Promise<T, E> {
}

pub fn reject<T, E>(val: E) -> Promise<T, E> {
}

#[test]
fn test_channels() {
    println!("Begin test...");
    let mut guard: Option<JoinHandle<_>> = None;
    let mut rec: Option<Receiver<_>> = None;

    {
        println!("\tBegin inner scope...");
        let (tx, rx) = channel();
        guard = Some(thread::spawn(move || {
            println!("\t\tThread created!");
            tx.send("Hello!").unwrap();
            println!("\t\tThread finished!");
        }));
        rec = Some(rx);
        println!("\t(tx, rx) out of scope");
    }

    println!("Sleeping...");
    thread::sleep(Duration::from_secs(2));
    println!("Receiving result...");
    let result = rec.unwrap().recv();
    assert_eq!(result, Ok("Hello!"));
}
