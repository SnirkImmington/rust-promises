//! Promises in Rust.

#![warn(incomplete_docs)]

use std::thread;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::thread::JoinHandle;
use std::sync::mpsc::{Sender, Receiver};

/// A promise is a way of doing work in the background
pub struct Promise<T, E> {
    //sender: Sender<PromiseMessage<T, E>>, // Send messages to the promise
    receiver: Receiver<Result<T, E>> // Get the result of the promise for chaining (pass into then)
}

enum PromiseJoinType {
    Race,
    All
}

impl<T, E> Promise<T, E> {

    /// Chains a function to be called after this promise resolves.
    pub fn then<T2, E2>(self, callback: fn(f: T) -> Result<T2, E2>,
                              errback:  fn(e: E) -> Result<T2, E2>)
                        -> Promise<T2, E2> {
        let recv = self.receiver;
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            if let Some(message) = recv.recv() { // Blocking receive until message
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
                               callback: fn(input: Result<T, E>) -> Result<T2, E2>),
                               -> Promise<T2, E2> {
        let recv = self.receiver;
        let recv2 = self.receiver;
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            if let Some(result) = recv.recv() { // Blocking receive until message
                let _ = tx.send(callback(result)).unwrap_or(());
            }
            else { // Receive failed, origin was dropped
            }
        });
        return Promise { receiver: rx };
    }


    pub fn new(func: fn() -> Result<T, E>) -> Promise<T, E> {
        let (tx, rx) = channel();

        let thread = thread::spawn(move || {
            let result = func();
            let _ = tx.send(func).unwrap_or(());
        });

        return Promise { receiver: rx };
    }
}

pub fn race<T, E>(promises: Vec<Promise<T, E>>) -> Promise<T, E> {
}

pub fn all<T, E>(promises: Vec<Promise<T, E>>) -> Promise<T, E> {
}


pub fn resolve<T, E>(val: E) -> Promise<T, E> {
}

pub fn reject<T, E>(val: E) -> Promise<T, E> {
}

// Dispatcher methods

fn spawn_vat<T, E>(func: fn(reciever: Receiver<PromiseMessage>) -> Result<T, E>) -> (Receiver<Result<T, E>>, JoinHandle<()>) {
    let (tx, rx) = channel();

}

fn vat_join() {
    
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
