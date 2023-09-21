# spawn_groups
[![rustc](https://img.shields.io/badge/rustc-1.70+-blue?style=flat-square&logo=rust)](https://www.rust-lang.org)

A structured concurrency construct which provides a way to spawn and run an arbitrary number of child tasks,
possibly await the results of each child task or even cancel all running child tasks.
It is influenced by the Swift language's [`TaskGroup`](https://developer.apple.com/documentation/swift/taskgroup) and Go language's [`errgroup`](https://pkg.go.dev/golang.org/x/sync/errgroup)

# Usage

To properly use this crate
* ``with_spawn_group`` for the creation of a dynamic number of asynchronous tasks that return a value. See [`with_spawn_group`](self::with_spawn_group)
for more information

* ``with_err_spawn_group`` for the creation of a dynamic number of asynchronous tasks that return a value or an error.
See [`with_err_spawn_group`](self::with_err_spawn_group)
for more information

* ``with_discarding_spawn_group`` for the creation of a dynamic number of asynchronous tasks that returns nothing.
See [`with_discarding_spawn_group`](self::with_discarding_spawn_group)
for more information

# Spawning Child Tasks

Child tasks are spawned by calling either `spawn_task` or `spawn_task_unless_cancelled` methods on any of the spawn groups' instance.

To avoid spawning new child tasks to an already cancelled spawn group, use ``spawn_task_unless_cancelled``
rather than the plain ``spawn_task`` which spawns new child tasks unconditionally.

# Child Task Execution Order

Child tasks spawned to a spawn group execute concurrently, and may be scheduled in
any order.
 
# Cancellation

By calling explicitly calling the ``cancel_all`` method on any of the spawn groups' instance, all child tasks
are immediately cancelled.

# Waiting

By calling explicitly calling the ``wait_for_all_tasks`` method on any of the spawn groups' instance, all child tasks
are immediately awaited for.

# Stream

Both [`SpawnGroup`](self::spawn_group::SpawnGroup) and [`ErrSpawnGroup`](self::err_spawn_group::ErrSpawnGroup) structs implements the ``futures_lite::Stream``
which means that you can await the result of each child task asynchronously and with the help of ``StreamExt`` trait, one can call methods such as ``next``,
``map``, ``filter_map``, ``fold`` and so much more.

If you want a specific number of results from the spawned child tasks,
consider calling ``get_chunks`` method instead of iterating over
the spawn group instance which waits for all child tasks to finish their execution

```rust
use spawn_groups::with_spawn_group;
use futures_lite::StreamExt;
use spawn_groups::Priority;
use spawn_groups::GetType;

with_spawn_group(i64::TYPE, |mut group| async move {
     for i in 0..=10 {
        group.spawn_task(Priority::default(), async move {
          // simulate asynchronous operation
             i
         });
     }

     // Loop over all the results of the child tasks spawned already
     while let Some(x) = group.next().await {
        println!("{}", x);
     }

}).await;
```

# Note

* Import ``StreamExt`` trait from ``futures_lite::StreamExt`` or ``futures::stream::StreamExt`` or ``async_std::stream::StreamExt`` to provide a variety of convenient combinator functions on the various spawn groups.
* To await all running child tasks to finish their execution, call ``wait_for_all`` method on the spawn group instance unless using the [`with_discarding_spawn_group`](self::with_discarding_spawn_group) function.

# Warning

* This crate relies on atomics
* Avoid using a spawn group from outside the above functions this crate provides
* Avoid calling long, blocking, non asynchronous functions while using any of the spawn groups because it was built with asynchrony in mind.
* Avoid spawning off an asynchronous function such as calling spawn methods from crate such as tokio, async_std, smol, etc.
