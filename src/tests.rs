#![cfg(test)]

// These tests involve async files
use std::fs;
use std::fs::File;
use std::thread;
use std::time::Duration;
use std::io;
use std::io::prelude::*;
use std::result::Result;

use super::Promise;

#[test]
pub fn test_new() {
    Promise::new(|| {
        File::create("/tmp/promise-new")
    });

    thread::sleep(Duration::from_secs(1));

    assert!(File::open("/tmp/promise-new").is_ok());
    fs::remove_file("/tmp/promise-new");
}

#[test]
pub fn test_then() {
    Promise::new(|| {
        fs::create_dir("/tmp/promise-then")
    })
    .then(|t| {
        File::create("/tmp/promise-then/file")
    }, |e| {
        println!("Unable to create dir!");
        Err(io::Error::new(io::ErrorKind::Other, "Couldn't make dir!"))
    });

    thread::sleep(Duration::from_secs(1));
    assert!(File::open("/tmp/promise-then/file").is_ok());
    fs::remove_dir_all("/tmp/promise-then");
}

#[derive(Debug)]
enum TestErrType {
    CreateDir,
    CreateFile,
    WriteFile,
    ReadFile
}

#[test]
pub fn test_then_ok() {
    Promise::new(|| {
        match fs::create_dir("/tmp/promise-then-ok/") {
            Ok(_) => Ok(()),
            Err(_) => Err(TestErrType::CreateDir)
        }
    })
    .then_ok(|k| {
        // k is (), as that's what was passed through Ok(())
        match File::create("/tmp/promise-then-ok/file") {
            Ok(file) => Ok(file), // Now file is passed along
            Err(_) => Err(TestErrType::CreateFile)
        }
    })
    // Note here mutability can be added in the scope of one promise's
    // function. This was not passed a mut file.
    .then_ok(|mut file| {
        match file.write_all(b"Hello world!") {
            Ok(_) => Ok(()), // Original file is dropped, flushes
            Err(_) => Err(TestErrType::WriteFile) // Keep err type
        }
    })
    .then_err(|e| {
        println!("File process errored at {:?}", e);
        Err(())
    });

    thread::sleep(Duration::from_secs(1));
    let mut s = String::new();
    let maybe_file = File::open("/tmp/promise-then-ok/file");
    assert!(maybe_file.is_ok());
    let mut file = maybe_file.unwrap();
    assert!(file.read_to_string(&mut s).is_ok());
    assert_eq!(s, "Hello world!");
}

#[test]
pub fn promise_all() {
    let mut p: Vec<Promise<u32,u32>> = Vec::new();
    for x in 0..10 {
        p.push(Promise::new(move || {
            thread::sleep(Duration::from_secs(1));
            Ok(x)
        })
        .then(move |t| {
            assert_eq!(t, x);
            Ok(t)
        }, |e| {
            println!("err: {:?}", e);
            Err(e)
        }));
    }

    Promise::all(p).then( move |t| {
      assert_eq!(t.len(), 10);
      Ok(())
    }, |e| {
      println!("err: {:?}", e);
      Err(())
    });
}
