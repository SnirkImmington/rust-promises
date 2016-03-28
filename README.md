# rust-promises
[![Crates.io](https://img.shields.io/badge/crates.io-v0.3-orange.svg)](https://crates.io/crate/promises)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/SnirkImmington/promises)
This library seeks to fully, effectively and simply recreate lightweight 
Javascript-style promises in Rust.

`rust-promises` offers all the features of Ecmascript 6 promises plus a few 
Rust methods from `Result<T, E>`, implemented using the `std::thread` API and 
Rust's asyncronous `Sender` and `Receiver`.

## Examples

TODO (after adding some `Result` methods to `Promise`).

## Why?

Promises are an excellent building block for asyncronous programming. They're
a great way to build an asyncronous library, and also a small dependency for
existing code.

Javascript's promises were adapted from promises in a programming language 
called [E](erights.org), which has a very powerful distributed computing
architecture using message passing. 

Rust's `std::sync::mpsc::{Sender, Receiver}` provide a way to build such an
architecture (and promises) from Rust's safe threading systems. 

### Oh great, another promise library
There are currently 5 asyncronous libraries on crates.io which offer promises
or futures. However they all have problems:
* ~~They are all in beta and unstable, with versions < 0.5~~ (like this library
right now...)
* Some of them are larger async libraries. They require programming within 
event loops, or are designed for specific scenarios. If you want a whole async
library for a specific type of programming, go use those.
* The rest are variations on the promise concept, i.e. returning a `(Future, Promise)` pair from `Promise::new()`. If you know how to use that, go to those libraries, I guess.
