use bytes::{Buf, BufMut, Bytes, BytesMut};
use memchr::{memchr, memchr2};
use std::{error::Error as StdError, fmt, str};

const CR: u8 = b'\r';
const LF: u8 = b'\n';
const COLON: u8 = b':';
const NULL: char = '\u{0000}';

/// Inner Error kind that contains possible errors occuring during parsing.
#[derive(Clone, Copy, Debug)]
pub enum Error {
    Utf8(std::str::Utf8Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Utf8(err) => write!(f, "Invalid UTF8: {}", err),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Utf8(ref err) => Some(err),
        }
    }
}

#[derive(Default)]
struct EventBuilder {
    event_type: Option<String>,
    data: Option<String>,
    last_event_id: Option<String>,
}

impl EventBuilder {
    pub fn add_field(&mut self, name: &[u8], value_bs: &[u8]) -> Result<(), Error> {
        let value = str::from_utf8(value_bs).map_err(Error::Utf8)?;

        if name == &b"event"[..] {
            // Set event type buffer to value. After parsing as utf8.
            self.event_type.replace(String::from(value));
        } else if name == &b"data"[..] {
            // According to the spec
            // (https://html.spec.whatwg.org/multipage/server-sent-events.html#event-stream-interpretation)
            // Whenever data is pushed, a single LF should be appended
            // and then removed whenever an entire event is created.
            // However this is stupid, better just add a LF before
            // appending data to an already existing data buffer.
            // So we push a LF before we add MORE data.
            match &mut self.data {
                Some(ref mut data) => {
                    data.reserve(value.len() + 1);
                    data.push('\n');
                    data.push_str(value);
                }

                None => {
                    self.data = Some(String::from(value));
                }
            }
        } else if name == &b"id"[..] && !value.contains(NULL) {
            // Set the latest_event_id buffer field.
            // value must not contain any nulls.
            self.last_event_id = Some(String::from(value));
        } else if name == &b"retry"[..] && value.chars().all(|c| c.is_digit(10)) {
            // If the field name is retry and the value is all base 10 digits.
            // use the value as the amount of time to wait before reconnects.
            // @TODO: Implement reconnection time.
        }

        Ok(())
    }

    fn ready(&self) -> bool {
        self.event_type.is_some() || self.data.is_some() || self.last_event_id.is_some()
    }

    fn build_and_clear(&mut self) -> Result<crate::Event, Error> {
        Ok(crate::Event {
            event: self.event_type.take().unwrap_or_else(String::new),
            data: self.data.take().unwrap_or_else(String::new),
            last_event_id: self.last_event_id.take(),
        })
    }
}

#[derive(Default)]
pub struct Parser {
    buf: BytesMut,
    builder: EventBuilder,
}

impl Parser {
    pub fn put(&mut self, bs: impl Buf) {
        self.buf.put(bs)
    }

    /// Parses a line and attemps to add it to the current Builder.
    ///
    pub fn next(&mut self) -> Option<Result<crate::Event, Error>> {
        // Parse while there are lines.

        while let Some(line) = self.parse_line() {
            if line.is_empty() && self.builder.ready() {
                return Some(self.builder.build_and_clear());
            }

            // Check if there's a colon in the line
            match memchr(COLON, &line) {
                // Lines beginning with colon are just skipped
                Some(0) => {
                    continue;
                }

                Some(i) => {
                    // name is all the characters to the left of the colon.
                    let name = &line[0..i];

                    // Let value be all the chars AFTER the colon.
                    // Drop any SPACE immeadately after the colon.
                    let value = if i + 1 < line.len() && line[i + 1] == b' ' {
                        &line[i + 2..]
                    } else {
                        &line[i + 1..]
                    };

                    // TODO:
                    // 1. Remove potential white space after colon
                    // 2. Verify that lines ending in colon works.
                    if let Err(err) = self.builder.add_field(name, value) {
                        return Some(Err(err));
                    }
                }

                None => {
                    if let Err(err) = self.builder.add_field(&line[..], &[][..]) {
                        return Some(Err(err));
                    }
                }
            }
        }

        None
    }

    fn parse_line(&mut self) -> Option<Bytes> {
        // Ways a line can end:
        //
        // a U+000D CARRIAGE RETURN U+000A LINE FEED (CRLF) character pair,
        // CRLF
        //
        // a single U+000A LINE FEED (LF) character not preceded by a
        // U+000D CARRIAGE RETURN (CR) character,
        // LF
        //
        // a single U+000D CARRIAGE RETURN (CR) character
        // not followed by a U+000A LINE FEED (LF) character
        // CR
        //
        // being the ways in which a line can end.

        match memchr2(CR, LF, &self.buf) {
            Some(i) => {
                let line = self.buf.split_to(i);

                if !self.buf.is_empty() {
                    if 2 < self.buf.len() && self.buf[0..2] == [CR, LF] {
                        self.buf.advance(2);
                    } else {
                        self.buf.advance(1);
                    }
                }

                Some(line.freeze())
            }

            None => None,
        }
    }

    #[cfg(test)]
    /// Helper fn for tests.
    fn bytes(&self) -> &[u8] {
        &self.buf
    }
}

impl From<&[u8]> for Parser {
    fn from(b: &[u8]) -> Self {
        Self {
            buf: BytesMut::from(b),
            builder: EventBuilder::default(),
        }
    }
}

impl From<&str> for Parser {
    fn from(s: &str) -> Self {
        Self {
            buf: BytesMut::from(s),
            builder: EventBuilder::default(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn buf_cleared_line_ending_with_crlf() {
        let mut p = Parser::from("\r\n");
        p.next();
        assert_eq!(p.bytes(), &[]);
    }

    #[test]
    fn single_lf_should_be_empty_line() {
        let mut p = Parser::from("\n");
        assert_eq!(p.parse_line().expect("parsing line"), &b""[..]);
    }

    #[test]
    fn buf_cleared_line_ending_with_cr() {
        let mut p = Parser::from("\r");
        p.next();
        assert_eq!(p.bytes(), &[]);
    }

    #[test]
    fn buf_cleared_line_ending_with_lf() {
        let mut p = Parser::from("\n");
        p.next();
        assert_eq!(p.bytes(), &[]);
    }

    #[test]
    fn lines_starting_with_colon_are_ignored() {
        let mut p = Parser::from(":ok");
        assert!(p.next().is_none());
    }

    #[test]
    fn test_memchr_order() {
        let bs = &b"abcd\r\n"[..];
        assert_eq!(memchr2(CR, LF, bs), Some(4));
        assert_eq!(memchr2(LF, CR, bs), Some(4));
    }

    #[test]
    fn colon_as_last_char_in_row() {
        let mut p = Parser::from("data:\n\n");
        let ev = p.next().expect("Expected an event").expect("Should parse");
        assert_eq!(ev.event, "");
        assert_eq!(ev.data, "");
    }

    #[test]
    fn parse_example_2_events() {
        // The following stream fires two events:

        let mut p = Parser::from(
            r#"
data

data
data

data:"#,
        );

        // The first block fires events with the data set to the empty string,
        // as would the last block if it was followed by a blank line.
        //
        // The middle block fires an event with the data set to a single newline character.
        //
        // The last block is discarded because it is not followed by a blank line.

        let ev = p.next().expect("Event").expect("Parsed");
        assert_eq!(ev.event, "");
        assert_eq!(ev.data, "");

        let ev = p.next().expect("Event").expect("Parsed");
        assert_eq!(ev.event, "");
        assert_eq!(ev.data, "\n");

        assert!(p.next().is_none());
    }

    #[test]
    fn parse_two_identical_events() {
        // The following stream fires two identical events:
        // This is because the space after the colon is ignored if present.
        let mut p = Parser::from(
            r#"
data:test

data: test

"#,
        );

        let ev = p
            .next()
            .expect("Expected first event")
            .expect("Should parse");

        assert_eq!(ev.event, "");
        assert_eq!(ev.data, "test");

        let ev = p
            .next()
            .expect("Expected first event")
            .expect("Should parse");

        assert_eq!(ev.event, "");
        assert_eq!(ev.data, "test");
    }

    #[test]
    fn parse_biggest_example_from_spec_page() {
        // The following stream contains four blocks.
        // The first block has just a comment, and will fire nothing.
        //
        // The second block has two fields with names "data" and "id" respectively;
        // an event will be fired for this block,
        // with the data "first event",
        // and will then set the last event ID to "1"
        // so that if the connection died between this block and the next,
        // the server would be sent a `Last-Event-ID` header with the value "1".
        //
        // The third block fires an event with data "second event", and also has an "id" field,
        // this time with no value, which resets the last event ID to the
        // empty string (meaning no `Last-Event-ID` header will now be sent in
        // the event of a reconnection being attempted).
        //
        // Finally, the last block just fires an event with the data " third event"
        // (with a single leading space character).
        // Note that the last still has to end with a blank line,
        // the end of the stream is not enough to trigger the dispatch of the last event.

        let mut p = Parser::from(
            r#"
: test stream

data: first event
id: 1

data:second event
id

data:  third event

"#,
        );

        let ev = p.next().expect("Event").expect("Parses");
        assert_eq!(ev.data, "first event");
        assert_eq!(ev.last_event_id.as_deref(), Some("1"));

        let ev = p.next().expect("Event").expect("Parses");
        assert_eq!(ev.data, "second event");
        assert_eq!(ev.last_event_id.as_deref(), Some(""));

        let ev = p.next().expect("Event").expect("Parses");
        assert_eq!(ev.data, " third event");
        assert_eq!(ev.last_event_id, None);
    }

    #[test]
    fn buf_fiddle() {
        let mut buf = BytesMut::from("1234");

        let left = buf.split_to(1);
        assert_eq!(left, &b"1"[..]);
        assert_eq!(buf, &b"234"[..]);
    }
}
