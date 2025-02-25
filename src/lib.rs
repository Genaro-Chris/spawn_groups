//! A structured concurrency construct which provides a way to spawn and run an arbitrary number of child tasks,
//! possibly await the results of each child task or even cancel all running child tasks.
//! This was heavily influenced by the Swift language's [`TaskGroup`](https://developer.apple.com/documentation/swift/taskgroup).
//!
//! # Installation
//! Add to your source code
//!
//! ```sh
//! cargo add spawn_groups
//! ```
//!
//! # Example
//!
//! ```ignore
//! use async_std::stream::StreamExt;
//! use spawn_groups::{with_err_spawn_group, GetType, Priority};
//! use std::time::Instant;
//! use surf::{Error, Client, http::Mime, StatusCode};
//!
//! async fn get_mimetype<AsStr: AsRef<str>>(url: AsStr, client: Client) -> Option<Mime> {
//!     let Ok(resp) = client.get(url).send().await else {
//!         return None;
//!     };
//!     resp.content_type()
//! }
//!
//! #[async_std::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = surf::Client::new();
//!     let urls = [
//!         "https://www.google.com",
//!         "https://www.bing.com",
//!         "https://www.yandex.com",
//!         "https://www.duckduckgo.com",
//!         "https://www.wikipedia.org",
//!         "https://www.whatsapp.com",
//!         "https://www.yahoo.com",
//!         "https://www.amazon.com",
//!         "https://www.baidu.com",
//!         "https://www.youtube.com",
//!         "https://facebook.com",
//!         "https://www.instagram.com",
//!         "https://tiktok.com",
//!     ];
//!     with_err_spawn_group(move |mut group| async move {
//!         println!("About to start");
//!         let now = Instant::now();
//!         for url in urls {
//!             let client = client.clone();
//!             group.spawn_task(Priority::default(), async move {
//!                 let Some(mimetype) = get_mimetype(url, client).await else {
//!                     return Err(Error::from_str(StatusCode::ExpectationFailed, format!("No content type found for {}", url)));
//!                 }
//!                 Ok(format!("{url}: {}", mimetype))
//!             })
//!         }
//!
//!         while let Some(result) = group.next().await {
//!             if let Err(error) = result {
//!                 eprintln!("{}", error);
//!             } else {
//!                 println!("{}", result.unwrap());
//!             }
//!         }
//!         println("Ended");
//!         println!("It took {} nanoseconds", now.elapsed().as_nanos());
//!     })
//!     .await;
//!     Ok(())
//! }
//! ```
//!
//! # Usage
//!
//! To properly use this crate
//! * ``with_spawn_group`` for the creation of a dynamic number of asynchronous tasks that return a value. See [`with_spawn_group`](self::with_spawn_group)
//! for more information
//!
//! * ``with_type_spawn_group`` for the creation of a dynamic number of asynchronous tasks that return a value by specifying the type explicitly. See [`with_type_spawn_group`](self::with_type_spawn_group)
//! for more information
//!
//! * ``with_err_spawn_group`` for the creation of a dynamic number of asynchronous tasks that return a value or an error.
//! See [`with_err_spawn_group`](self::with_err_spawn_group)
//! for more information
//!
//! * ``with_err_type_spawn_group`` for the creation of a dynamic number of asynchronous tasks that return a value or an error by specifiying the return type and the error type explicitly.
//! See [`with_err_type_spawn_group`](self::with_err_type_spawn_group)
//! for more information
//!
//! * ``with_discarding_spawn_group`` for the creation of a dynamic number of asynchronous tasks that returns nothing.
//! See [`with_discarding_spawn_group`](self::with_discarding_spawn_group)
//! for more information
//!
//! * ``block_on`` polls future to finish. See [`block_on`](self::block_on)
//! for more information
//!
//! # Spawning Child Tasks
//!
//! Child tasks are spawned by calling either `spawn_task` or `spawn_task_unless_cancelled` methods on any of the spawn groups' instance.
//!
//! To avoid spawning new child tasks to an already cancelled spawn group, use ``spawn_task_unless_cancelled``
//! rather than the plain ``spawn_task`` which spawns new child tasks unconditionally.
//!
//! # Child Task Execution Order
//! Child tasks are scheduled in any order and spawned child tasks execute concurrently.
//!  
//! # Cancellation
//!
//! By calling explicitly calling the ``cancel_all`` method on any of the spawn groups' instance, all running child tasks
//! are immediately cancelled.
//!
//! # Waiting
//!
//! By calling explicitly calling the ``wait_for_all_tasks`` method on any of the spawn groups' instance, all child tasks
//! are immediately awaited for.
//!
//! # Stream
//!
//! Both [`SpawnGroup`](self::spawn_group::SpawnGroup) and [`ErrSpawnGroup`](self::err_spawn_group::ErrSpawnGroup) structs implements the ``futures_lite::Stream``
//! which means that you can await the result of each child task asynchronously and with the help of ``StreamExt`` trait, one can call methods such as ``next``,
//! ``map``, ``filter_map``, ``fold`` and so much more.
//!
//! ```rust
//! use spawn_groups::with_spawn_group;
//! use futures_lite::StreamExt;
//! use spawn_groups::Priority;
//! use spawn_groups::GetType;
//!
//! # spawn_groups::block_on(async move {
//! with_spawn_group(|mut group| async move {
//!      for i in 0..=10 {
//!         group.spawn_task(Priority::default(), async move {
//!           // simulate asynchronous operation
//!              i
//!          });
//!      }
//!
//!      // Loop over all the results of the child tasks spawned already
//!      let mut counter = 0;
//!      while let Some(x) = group.next().await {
//!         counter += x;
//!      }
//!
//!     assert_eq!(counter, 55);
//!
//! }).await;
//! # });
//!
//! ```
//!
//! # Comparisons against existing alternatives
//!
//!
//!
//! # Note
//! * Import ``StreamExt`` trait from ``futures_lite::StreamExt`` or ``futures::stream::StreamExt`` or ``async_std::stream::StreamExt`` to provide a variety of convenient combinator functions on the various spawn groups.
//! * To await all running child tasks to finish their execution, call ``wait_for_all`` or ``wait_non_async`` methods on the various group instances
//!
//! # Warning
//! * This crate relies on atomics
//! * Avoid using a spawn group from outside the above functions this crate provides
//! * Avoid calling long, blocking, non asynchronous functions while using any of the spawn groups because it was built with asynchrony in mind.
//! * Avoid spawning off an asynchronous function such as calling spawn methods from crate such as tokio, async_std, smol, etc.

mod discarding_spawn_group;
mod err_spawn_group;
mod spawn_group;

mod async_stream;
mod executors;
mod meta_types;
mod shared;
mod threadpool_impl;

pub use discarding_spawn_group::DiscardingSpawnGroup;
pub use err_spawn_group::ErrSpawnGroup;
pub use executors::block_on;
pub use meta_types::GetType;
pub use shared::priority::Priority;
pub use spawn_group::SpawnGroup;

use std::future::Future;
use std::marker::PhantomData;

/// Starts a scoped closure that takes a mutable ``SpawnGroup`` instance as an argument which can execute any number of child tasks which its result values are of the generic ``ResultType`` type.
///
/// This closure ensures that before the function call ends, all spawned child tasks are implicitly waited for, or the programmer can explicitly wait by calling  its ``wait_for_all()`` method
/// of the ``SpawnGroup`` struct.
///
/// This function use a threadpool of the same number of threads as the number of active processor count that is default amount of parallelism a program can use on the system for polling the spawned tasks
///
/// See [`SpawnGroup`](spawn_group::SpawnGroup)
/// for more.
///
/// # Parameters
///
/// * `of_type`: The type which the child task can return
/// * `body`: an async closure that takes a mutable instance of ``SpawnGroup`` as an argument
///
/// # Returns
///
/// Anything the ``body`` parameter returns
///
/// # Example
///
/// ```rust
/// use spawn_groups::GetType;
/// use spawn_groups::with_type_spawn_group;
/// use futures_lite::StreamExt;
/// use spawn_groups::Priority;
///
/// # spawn_groups::block_on(async move {
/// let final_result = with_type_spawn_group(i64::TYPE, |mut group| async move {
///      for i in 0..=10 {
///         group.spawn_task(Priority::default(), async move {
///            // simulate asynchronous operation
///            i
///         });
///      }
///
///      group.fold(0, |acc, x| {
///          acc + x
///      }).await
///  }).await;
///
///  assert_eq!(final_result, 55);
/// # });
/// ```
pub async fn with_type_spawn_group<Closure, Fut, ResultType, ReturnType>(
    of_type: PhantomData<ResultType>,
    body: Closure,
) -> ReturnType
where
    Closure: FnOnce(spawn_group::SpawnGroup<ResultType>) -> Fut,
    Fut: Future<Output = ReturnType> + 'static,
    ResultType: 'static,
{
    _ = of_type;
    let task_group = spawn_group::SpawnGroup::<ResultType>::default();
    body(task_group).await
}

/// Starts a scoped closure that takes a mutable ``SpawnGroup`` instance as an argument which can execute any number of child tasks which its result values are of the generic ``ResultType`` type.
///
/// This closure ensures that before the function call ends, all spawned child tasks are implicitly waited for, or the programmer can explicitly wait by calling  its ``wait_for_all()`` method
/// of the ``SpawnGroup`` struct.
///
/// This function use a threadpool of the same number of threads as the number of active processor count that is default amount of parallelism a program can use on the system for polling the spawned tasks
///
/// See [`SpawnGroup`](spawn_group::SpawnGroup)
/// for more.
///
/// # Parameters
///
/// * `body`: an async closure that takes a mutable instance of ``SpawnGroup`` as an argument
///
/// # Returns
///
/// Anything the ``body`` parameter returns
///
/// # Example
///
/// ```rust
/// use spawn_groups::GetType;
/// use spawn_groups::with_spawn_group;
/// use futures_lite::StreamExt;
/// use spawn_groups::Priority;
///
/// # spawn_groups::block_on(async move {
/// let final_result = with_spawn_group(|mut group| async move {
///      for i in 0..=10 {
///         group.spawn_task(Priority::default(), async move {
///            // simulate asynchronous operation
///            i
///         });
///      }
///
///      group.fold(0, |acc, x| {
///          acc + x
///      }).await
///  }).await;
///
///  assert_eq!(final_result, 55);
/// # });
/// ```
pub async fn with_spawn_group<Closure, Fut, ResultType, ReturnType>(body: Closure) -> ReturnType
where
    Closure: FnOnce(spawn_group::SpawnGroup<ResultType>) -> Fut,
    Fut: Future<Output = ReturnType> + 'static,
    ResultType: 'static,
{
    let task_group = spawn_group::SpawnGroup::<ResultType>::default();
    body(task_group).await
}

/// Starts a scoped closure that takes a mutable ``ErrSpawnGroup`` instance as an argument which can execute any number of child tasks which its result values are of the type ``Result<ResultType, ErrorType>``
/// where ``ResultType`` can be of type and ``ErrorType`` which is any type that implements the standard ``Error`` type.
///
/// This closure ensures that before the function call ends, all spawned child tasks are implicitly waited for, or the programmer can explicitly wait by calling its ``wait_for_all()`` method
/// of the ``ErrSpawnGroup`` struct
///
/// This function use a threadpool of the same number of threads as the number of active processor count that is default amount of parallelism a program can use on the system for polling the spawned tasks
///
/// See [`ErrSpawnGroup`](err_spawn_group::ErrSpawnGroup)
/// for more.
///
/// # Parameters
///
/// * `of_type`: The type which the child task can return
/// * `error_type`: The error type which the child task can return
/// * `body`: an async closure that takes a mutable instance of ``ErrSpawnGroup`` as an argument
///
/// # Returns
///
/// Anything the ``body`` parameter returns
///
/// # Example
///
/// ```rust
/// use std::error::Error;
/// use std::fmt::Display;
/// use spawn_groups::GetType;
/// use spawn_groups::with_err_type_spawn_group;
/// use futures_lite::StreamExt;
/// use spawn_groups::Priority;
///
/// #[derive(Debug)]
/// enum DivisibleByError {
///     THREE,
///     FIVE
/// }
///
/// impl Display for DivisibleByError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             DivisibleByError::THREE => f.write_str("Divisible by three"),
///             DivisibleByError::FIVE => f.write_str("Divisible by five")
///         }
///     }
/// }
///
/// impl Error for DivisibleByError {}
///
/// # spawn_groups::block_on(async move {
/// let final_results = with_err_type_spawn_group(u8::TYPE, DivisibleByError::TYPE, |mut group| async move {
///     for i in 1..=10 {
///         group.spawn_task(Priority::default(), async move {
///          // simulate asynchronous processing that might fail and
///          // return a value of ErrorType specified above
///             if i % 3 == 0 {
///                return Err(DivisibleByError::THREE)
///             } else if i % 5 == 0 {
///                return Err(DivisibleByError::FIVE)
///             }
///             Ok(i)
///           });
///        }
///             
///   // Explicitly wait for the all spawned child tasks to finish
///     group.wait_for_all().await;
///
///     let mut sum_result = 0;
///     let mut divisible_by_five = 0;
///     let mut divisible_by_three = 0;
///     while let Some(group_result) = group.next().await {
///        if let Ok(result) = group_result {
///          sum_result += result;
///        } else if let Err(err_result) = group_result {
///            match err_result {
///              DivisibleByError::THREE => divisible_by_three += 1,
///              DivisibleByError::FIVE => divisible_by_five += 1
///            }
///        }
///     }
///
///     (sum_result, divisible_by_three, divisible_by_five)
///
/// }).await;
///
/// assert_eq!(final_results.0, 22);
/// assert_eq!(final_results.1, 3);
/// assert_eq!(final_results.2, 2);
/// # });
/// ```
pub async fn with_err_type_spawn_group<Closure, Fut, ResultType, ErrorType, ReturnType>(
    of_type: PhantomData<ResultType>,
    error_type: PhantomData<ErrorType>,
    body: Closure,
) -> ReturnType
where
    ErrorType: 'static,
    Fut: Future<Output = ReturnType>,
    Closure: FnOnce(err_spawn_group::ErrSpawnGroup<ResultType, ErrorType>) -> Fut,
    ResultType: 'static,
{
    _ = (of_type, error_type);
    let task_group = err_spawn_group::ErrSpawnGroup::<ResultType, ErrorType>::default();
    body(task_group).await
}

/// Starts a scoped closure that takes a mutable ``ErrSpawnGroup`` instance as an argument which can execute any number of child tasks which its result values are of the type ``Result<ResultType, ErrorType>``
/// where ``ResultType`` can be of type and ``ErrorType`` which is any type that implements the standard ``Error`` type.
///
/// This closure ensures that before the function call ends, all spawned child tasks are implicitly waited for, or the programmer can explicitly wait by calling its ``wait_for_all()`` method
/// of the ``ErrSpawnGroup`` struct
///
/// This function use a threadpoolof the same number of threads as the number of active processor count that is default amount of parallelism a program can use on the system  for polling the spawned tasks
///
/// See [`ErrSpawnGroup`](err_spawn_group::ErrSpawnGroup)
/// for more.
///
/// # Parameters
///
/// * `body`: an async closure that takes a mutable instance of ``ErrSpawnGroup`` as an argument
///
/// # Returns
///
/// Anything the ``body`` parameter returns
///
/// # Example
///
/// ```rust
/// use std::error::Error;
/// use std::fmt::Display;
/// use spawn_groups::GetType;
/// use spawn_groups::with_err_spawn_group;
/// use futures_lite::StreamExt;
/// use spawn_groups::Priority;
///
/// #[derive(Debug)]
/// enum DivisibleByError {
///     THREE,
///     FIVE
/// }
///
/// impl Display for DivisibleByError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             DivisibleByError::THREE => f.write_str("Divisible by three"),
///             DivisibleByError::FIVE => f.write_str("Divisible by five")
///         }
///     }
/// }
///
/// impl Error for DivisibleByError {}
///
/// # spawn_groups::block_on(async move {
/// let final_results = with_err_spawn_group(|mut group| async move {
///     for i in 1..=10 {
///         group.spawn_task(Priority::default(), async move {
///          // simulate asynchronous processing that might fail and
///          // return a value of ErrorType specified above
///             if i % 3 == 0 {
///                return Err(DivisibleByError::THREE)
///             } else if i % 5 == 0 {
///                return Err(DivisibleByError::FIVE)
///             }
///             Ok(i)
///           });
///        }
///             
///   // Explicitly wait for the all spawned child tasks to finish
///     group.wait_for_all().await;
///
///     let mut sum_result = 0;
///     let mut divisible_by_five = 0;
///     let mut divisible_by_three = 0;
///     while let Some(group_result) = group.next().await {
///        if let Ok(result) = group_result {
///          sum_result += result;
///        } else if let Err(err_result) = group_result {
///            match err_result {
///              DivisibleByError::THREE => divisible_by_three += 1,
///              DivisibleByError::FIVE => divisible_by_five += 1
///            }
///        }
///     }
///
///     (sum_result, divisible_by_three, divisible_by_five)
///
/// }).await;
///
/// assert_eq!(final_results.0, 22);
/// assert_eq!(final_results.1, 3);
/// assert_eq!(final_results.2, 2);
/// # });
/// ```
pub async fn with_err_spawn_group<Closure, Fut, ResultType, ErrorType, ReturnType>(
    body: Closure,
) -> ReturnType
where
    ErrorType: 'static,
    Fut: Future<Output = ReturnType>,
    Closure: FnOnce(err_spawn_group::ErrSpawnGroup<ResultType, ErrorType>) -> Fut,
    ResultType: 'static,
{
    let task_group = err_spawn_group::ErrSpawnGroup::<ResultType, ErrorType>::default();
    body(task_group).await
}

/// Starts a scoped closure that takes a mutable ``DiscardingSpawnGroup`` instance as an argument which can execute any number of child tasks which return nothing.
///
/// Ensures that before the function call ends, all spawned tasks are implicitly waited for
///
/// This function use a threadpool of the same number of threads as the number of active processor count that is default amount of parallelism a program can use on the system for polling the spawned tasks
///
/// See [`DiscardingSpawnGroup`](discarding_spawn_group::DiscardingSpawnGroup)
/// for more.
///
/// # Parameters
///
/// * `body`: an async closure that takes an instance of ``DiscardingSpawnGroup`` as an argument
///
/// # Returns
///
/// Anything the ``body`` parameter returns
///   
/// # Example
///
/// ```rust
/// use spawn_groups::GetType;
/// use spawn_groups::with_discarding_spawn_group;
/// use futures_lite::StreamExt;
/// use spawn_groups::Priority;
///
/// # spawn_groups::block_on(async move {
/// with_discarding_spawn_group(|mut group| async move {
///     for i in 0..11 {
///        group.spawn_task(Priority::default(), async move {
///         // asynchronous processing
///         // or some async network calls
///        });
///     }
///
/// }).await;
/// # });
/// ```
pub async fn with_discarding_spawn_group<Closure, Fut, ReturnType>(body: Closure) -> ReturnType
where
    Fut: Future<Output = ReturnType>,
    Closure: FnOnce(discarding_spawn_group::DiscardingSpawnGroup) -> Fut,
{
    let discarding_tg = discarding_spawn_group::DiscardingSpawnGroup::default();
    body(discarding_tg).await
}
