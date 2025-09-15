# spawn_groups

[![rustc](https://img.shields.io/badge/rustc-1.70+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![crate](https://img.shields.io/docsrs/spawn_groups)](https://docs.rs/spawn_groups/1.0.0)
[![github](https://img.shields.io/badge/spawn_group-grey?logo=Github&logoColor=white&label=github&labelColor=black)](https://github.com/Genaro-Chris/spawn_groups)
[![license](https://img.shields.io/github/license/Genaro-Chris/spawn_groups)]()

## Introduction

A structured concurrency construct which provides a way to spawn and run an arbitrary number of child tasks,
possibly await the results of each child task immediately after it has finish executing, cancel all running child tasks or wait for all child tasks to finish their execution.
This was heavily influenced by the Swift language's [`TaskGroup`](https://developer.apple.com/documentation/swift/taskgroup).

## Installation

Add to your code

```sh
cargo add spawn_groups@1.1.0
```

## Example

```rust
use async_std::stream::StreamExt;
use spawn_groups::{with_err_spawn_group, GetType, Priority};
use std::time::Instant;
use surf::{Error, Client, http::Mime, StatusCode};

async fn get_mimetype<AsStr: AsRef<str>>(url: AsStr, client: Client) -> Option<Mime> {
    let Ok(resp) = client.get(url).send().await else {
        return None;
    };
    resp.content_type()
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = surf::Client::new();
    let urls = [
        "https://www.google.com",
        "https://www.bing.com",
        "https://www.yandex.com",
        "https://www.duckduckgo.com",
        "https://www.wikipedia.org",
        "https://www.whatsapp.com",
        "https://www.yahoo.com",
        "https://www.amazon.com",
        "https://www.baidu.com",
        "https://www.youtube.com",
        "https://facebook.com",
        "https://www.instagram.com",
        "https://tiktok.com",
    ];
    with_err_spawn_group(String::TYPE, Error::TYPE, move |mut group| async move {
        println!("About to start");
        let now = Instant::now();
        for url in urls {
            let client = client.clone();
            group.spawn_task(Priority::default(), async move {
                if let Some(mimetype) = get_mimetype(url, client).await {
                    return Ok(format!("{url}: {}", mimetype));
                }
                Err(Error::from_str(StatusCode::ExpectationFailed, format!("No content type found for {}", url)))
            })
        }

        while let Some(result) = group.next().await {
            if let Err(error) = result {
                eprintln!("{}", error);
            } else {
                println!("{}", result.unwrap());
            }
        }
        println!("It took {} nanoseconds", now.elapsed().as_nanos());
    })
    .await;
    Ok(())
}
```

## Documentation

For a better documentation of this rust crate. Visit [here](https://docs.rs/spawn_groups/latest)


# Comparison against existing alternatives

* [`JoinSet`](): Like this alternative, both await the completion of some or all of the child tasks, spawn child tasks in an unordered manner and the result of their child tasks will be returned in the order they complete and also cancel or abort all child tasks. Unlike the `Joinset`, you can explicitly await for all the child task to finish their execution. The Spawn group option provides a scope for the child tasks to execute.

* [`FuturesUnordered`]() Like this alternative, both spawn child tasks in an unordered manner, but FuturesUnordered doesn't immediately start running the spawned child tasks until it is being polled. It also doesn't provide a way to cancel all child tasks. 
