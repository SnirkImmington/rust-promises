//! Promises in Rust.

#![warn(missing_docs)]

use std::thread;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::thread::JoinHandle;
use std::marker::{Send};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};

type Promiseable = Send + 'static;

/// A promise is a way of doing work in the background
pub struct Promise<T, E> where T: Send, E: Send {
    receiver: Receiver<Result<T, E>> // Get the result of the promise for chaining (pass into then)
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
