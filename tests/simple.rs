use futures_util::StreamExt;
use sse_agent::SseBody;

#[tokio::test]
async fn parse_hyper_body_as_sse() {
    let mut sse = hyper::Body::from(
        r#"
: test stream

data: first event
id: 1

data:second event
id

data:  third event

"#,
    )
    .into_sse();

    let ev = sse.next().await.expect("Event").expect("Parses");
    assert_eq!(ev.data, "first event");
    assert_eq!(ev.last_event_id.as_deref(), Some("1"));

    let ev = sse.next().await.expect("Event").expect("Parses");
    assert_eq!(ev.data, "second event");
    assert_eq!(ev.last_event_id.as_deref(), Some(""));

    let ev = sse.next().await.expect("Event").expect("Parses");
    assert_eq!(ev.data, " third event");
    assert_eq!(ev.last_event_id, None);
}

#[tokio::test]
async fn parse_example_two_events() {
    // The following stream fires two events:

    let mut p = hyper::Body::from(
        r#"
data

data
data

data:"#,
    )
    .into_sse();

    // The first block fires events with the data set to the empty string,
    // as would the last block if it was followed by a blank line.
    //
    // The middle block fires an event with the data set to a single newline character.
    //
    // The last block is discarded because it is not followed by a blank line.

    let ev = p.next().await.expect("Event").expect("Parsed");
    assert_eq!(ev.event, "");
    assert_eq!(ev.data, "");

    let ev = p.next().await.expect("Event").expect("Parsed");
    assert_eq!(ev.event, "");
    assert_eq!(ev.data, "\n");

    assert!(p.next().await.is_none());
}
