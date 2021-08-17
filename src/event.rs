#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Event {
    pub event: String,
    pub data: String,
    pub last_event_id: Option<String>,
}
