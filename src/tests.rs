#![cfg(test)]

// These tests involve async files
use std::fs;
use std::fs::File;
use std::thread;
use std::time::Duration;
use std::io;
use std::result::Result;

use super::Promise;

fn create_file() -> Result<File, io::Error> {
    println!("In create_file");
    let res = File::create("/tmp/rust-promise-tests/new");
    println!("Finishing promise");
    res
}

fn handle_result(r: Result<File, io::Error>) -> Result<(), ()> {
    println!("Handling result");
    match r {
        Ok(file) => println!("Created a file!"),
        Err(e) => println!("Couldn't create a file: {}!", e)
    }
    return Ok(());
}

#[test]
pub fn test_new() {
    let file_name = "/tmp/rust-promise-tests/new";

    println!("Created promise");
    let p: Promise<(), ()> = Promise::new::<File>(create_file)
        .then_result(handle_result);

    thread::sleep(Duration::from_secs(1));

    println!("Checking file");
    //assert!(File::open(file_name).is_ok());
}
