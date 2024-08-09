/// Task Priority
///
/// Spawn groups uses it to rank the importance of their spawned tasks and order of returned values only when waited for
/// that is when the ``wait_for_all`` or ``wait_non_async`` methods are called
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Priority {
    BACKGROUND = 0,
    LOW,
    UTILITY,
    #[default]
    MEDIUM,
    HIGH,
    USERINITIATED,
}
