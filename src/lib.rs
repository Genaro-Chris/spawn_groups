//! A structured concurrency construct which provides a way to spawn and run an arbitrary number of child tasks,
//! possibly await the results of each child task or even cancel all running child tasks.
//! It is influenced by the Swift language's [`TaskGroup`](https://developer.apple.com/documentation/swift/taskgroup) 
//! and Go language's [`errgroup`](https://pkg.go.dev/golang.org/x/sync/errgroup)
//!
//! # Usage
//!
//! To properly use this crate
//! * ``with_spawn_group`` for the creation of a dynamic number of asynchronous tasks that return a value. See [`with_spawn_group`](self::with_spawn_group)
//! for more information
//!
//! * ``with_err_spawn_group`` for the creation of a dynamic number of asynchronous tasks that return a value or an error.
//! See [`with_err_spawn_group`](self::with_err_spawn_group)
//! for more information
//!
//! * ``with_discarding_spawn_group`` for the creation of a dynamic number of asynchronous tasks that returns nothing.
//! See [`with_discarding_spawn_group`](self::with_discarding_spawn_group)
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
//! Child tasks spawned to a spawn group execute concurrently, and may be scheduled in
//! any order.
//!  
//! # Cancellation
//!
//! By calling explicitly calling the ``cancel_all`` method on any of the spawn groups' instance, all child tasks
//! are immediately cancelled.
//!
//! # Waiting
//!
//! By calling explicitly calling the ``wait_for_all_tasks`` method on any of the spawn groups' instance, all child tasks
//! are immediately awaited for.
//!
//! # Stream
//! Both [`SpawnGroup`](self::spawn_group::SpawnGroup) and [`ErrSpawnGroup`](self::err_spawn_group::ErrSpawnGroup) structs implements the ``futures_lite::Stream``
//! which means that you can await the result of each child task asynchronously and with the help of ``StreamExt`` trait, one can call methods such as ``next``,
//! ``map``, ``filter_map``, ``fold`` and so much more.
//!
//! If you want a specific number of results from the spawned child tasks,
//! consider calling ``get_chunks`` method instead of iterating over
//! the spawn group instance which waits for all child tasks to finish their execution
//!
//! ```rust
//! use spawn_groups::with_spawn_group;
//! use futures_lite::StreamExt;
//! use spawn_groups::Priority;
//! use spawn_groups::GetType;
//!
//! # futures_lite::future::block_on(async move {
//! let final_result = with_spawn_group(i64::TYPE, |mut group| async move {
//!      for i in 0..=10 {
//!         group.spawn_task(Priority::default(), async move {
//!           // simulate asynchronous operation
//!              i
//!          });
//!      }
//!
//!      // Loop over all the results of the child tasks spawned already
//!      while let Some(x) = group.next().await {
//!         println!("{}", x);
//!      }
//!
//! }).await;
//! # });
//!
//! ```
//!
//! # Note
//! * Import ``StreamExt`` trait from ``futures_lite::StreamExt`` or ``futures::stream::StreamExt`` or ``async_std::stream::StreamExt`` to provide a variety of convenient combinator functions on the various spawn groups.
//! * To await all running child tasks to finish their execution, call ``wait_for_all`` method on the spawn group instance unless using the [`with_discarding_spawn_group`](self::with_discarding_spawn_group) function.
//! 
//! # Warning
//! * This crate relies on atomics
//! * Avoid using a spawn group from outside the above functions this crate provides
//! * Avoid calling long, blocking, non asynchronous functions while using any of the spawn groups because it was built with asynchrony in mind.
//! * Avoid spawning off an asynchronous function such as calling spawn methods from crate such as tokio, async_std, smol, etc.

pub mod discarding_spawn_group;
pub mod err_spawn_group;
pub mod meta_types;
pub mod spawn_group;

mod async_runtime;
mod async_stream;
mod shared;

pub use meta_types::GetType;
pub use shared::priority::Priority;

use std::future::Future;
use std::marker::PhantomData;

/// Starts a scoped closure that takes a mutable ``SpawnGroup`` instance as an argument which can execute any number of child tasks which its result values are of the generic ``ResultType`` type.
///
/// This closure ensures that before the function call ends, all spawned child tasks are implicitly waited for, or the programmer can explicitly wait by calling  its ``wait_for_all()`` method
/// of the ``SpawnGroup`` struct.
///
/// See [`SpawnGroup`](spawn_group::SpawnGroup)
/// for more.
///
/// # Parameters
///
/// * `of_type`: The type which the child task can return
/// * `body`: an async closure that takes a mutable instance of ``SpawnGroup`` as an argument
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
/// # futures_lite::future::block_on(async move {
/// let final_result = with_spawn_group(i64::TYPE, |mut group| async move {
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
pub async fn with_spawn_group<Closure, Fut, ResultType, ReturnType>(
    of_type: PhantomData<ResultType>,
    body: Closure,
) -> ReturnType
where
    Closure: FnOnce(spawn_group::SpawnGroup<ResultType>) -> Fut + Send + 'static,
    Fut: Future<Output = ReturnType> + Send + 'static,
    ResultType: Send + 'static,
{
    _ = of_type;
    let task_group = spawn_group::SpawnGroup::<ResultType>::new();
    body(task_group).await
}

/// Starts a scoped closure that takes a mutable ``ErrSpawnGroup`` instance as an argument which can execute any number of child tasks which its result values are of the type ``Result<ResultType, ErrorType>``
/// where ``ResultType`` can be of type and ``ErrorType`` which is any type that implements the standard ``Error`` type.
///
/// This closure ensures that before the function call ends, all spawned child tasks are implicitly waited for, or the programmer can explicitly wait by calling its ``wait_for_all()`` method
/// of the ``ErrSpawnGroup`` struct
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
/// # futures_lite::future::block_on(async move {
/// let final_results = with_err_spawn_group(u8::TYPE, DivisibleByError::TYPE, |mut group| async move {
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
///          sum_result += group_result.unwrap();
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
    of_type: PhantomData<ResultType>,
    error_type: PhantomData<ErrorType>,
    body: Closure,
) -> ReturnType
where
    ErrorType: std::error::Error + Send + 'static,
    Fut: Future<Output = ReturnType>,
    Closure: FnOnce(err_spawn_group::ErrSpawnGroup<ResultType, ErrorType>) -> Fut + Send + 'static,
    ResultType: Send + 'static,
{
    _ = (of_type, error_type);
    let task_group = err_spawn_group::ErrSpawnGroup::<ResultType, ErrorType>::new();
    body(task_group).await
}

/// Starts a scoped closure that takes a mutable ``DiscardingSpawnGroup`` instance as an argument which can execute any number of child tasks which return nothing.
///
/// Ensures that before the function call ends, all spawned tasks are implicitly waited for
///
/// See [`DiscardingSpawnGroup`](discarding_spawn_group::DiscardingSpawnGroup)
/// for more.
///
/// # Parameters
///
/// * `body`: an async closure that takes an instance of ``DiscardingSpawnGroup`` as an argument
///   
/// # Example
///
/// ```rust
/// use spawn_groups::GetType;
/// use spawn_groups::with_discarding_spawn_group;
/// use futures_lite::StreamExt;
/// use spawn_groups::Priority;
///
/// # futures_lite::future::block_on(async move {
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
pub async fn with_discarding_spawn_group<Closure, Fut>(body: Closure)
where
    Fut: Future<Output = ()>,
    Closure: FnOnce(discarding_spawn_group::DiscardingSpawnGroup) -> Fut + Send + 'static,
{
    let discarding_tg = discarding_spawn_group::DiscardingSpawnGroup::new();
    _ = body(discarding_tg).await;
}
