#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Event {
    pub typ: String,
    pub data: String,
    pub last_event_id: Option<String>,
}
