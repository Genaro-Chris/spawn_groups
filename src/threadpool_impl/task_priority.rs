use crate::Priority;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TaskPriority {
    Wait,
    Background,
    Low,
    Utility,
    Medium,
    High,
    UserInitiated,
}

impl From<Priority> for TaskPriority {
    fn from(value: Priority) -> Self {
        match value {
            Priority::BACKGROUND => TaskPriority::Background,
            Priority::LOW => TaskPriority::Low,
            Priority::UTILITY => TaskPriority::Utility,
            Priority::MEDIUM => TaskPriority::Medium,
            Priority::HIGH => TaskPriority::High,
            Priority::USERINITIATED => TaskPriority::UserInitiated,
        }
    }
}
