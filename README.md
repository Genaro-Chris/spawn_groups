# spawn_groups
[![rustc](https://img.shields.io/badge/rustc-1.70+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![crate](https://img.shields.io/docsrs/spawn_groups)](https://docs.rs/spawn_groups/0.1.0)
[![github](https://img.shields.io/badge/spawn_group-grey?logo=Github&logoColor=white&label=github&labelColor=black)](https://github.com/Genaro-Chris/spawn_groups)
[![license](https://img.shields.io/github/license/Genaro-Chris/spawn_groups)]()

# Introduction

A structured concurrency construct which provides a way to spawn and run an arbitrary number of child tasks,
possibly await the results of each child task immediately after it has finish executing, cancel all running child tasks or wait for all child tasks to finish their execution.
This was heavily influenced by the Swift language's [`TaskGroup`](https://developer.apple.com/documentation/swift/taskgroup).

# Installation
Add to your code 
****
```sh
$ cargo add spawn_groups
```

# Example
```rust
use async_std::io::{self};
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use spawn_groups::{with_err_spawn_group, GetType, Priority};

async fn process(stream: TcpStream) -> io::Result<()> {
    println!("Accepted from local: {}", stream.local_addr()?);
    println!("Accepted from: {}", stream.peer_addr()?);
    let mut reader = stream.clone();
    let mut writer = stream;
    io::copy(&mut reader, &mut writer).await?;
    Ok(())
}

type Void = ();

#[async_std::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    with_err_spawn_group(Void::TYPE, io::Error::TYPE, |mut group| async move {
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let Ok(stream) = stream else {
                return Err(stream.expect_err("Expected an error"));
            };
            group.spawn_task(Priority::default(), async move { process(stream).await });
        }
        Ok(())
    })
    .await?;

    Ok(())
}
```

# Documentation

For a better documentation of this rust crate. Visit [here](https://docs.rs/spawn_groups/1.0.0) 
