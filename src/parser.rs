use bytes::{Buf, BufMut, Bytes, BytesMut};
use memchr::{memchr, memchr2};
use std::str;

const CR: u8 = b'\r';
const LF: u8 = b'\n';
const COLON: u8 = b':';
const NULL: char = '\u{0000}';

/// Inner Error kind that contains possible errors occuring during parsing.
#[derive(Clone, Copy, Debug)]
pub enum Error {
    InvalidUtf8InValue,
}

#[derive(Default)]
struct EventBuilder {
    event_type: Option<String>,
    data: Option<String>,
    last_event_id: Option<String>,
}

impl EventBuilder {
    pub fn add_field(&mut self, name: &[u8], value_bs: &[u8]) -> Result<(), Error> {
        let value = str::from_utf8(value_bs).map_err(|_| Error::InvalidUtf8InValue)?;

        if name == &b"event"[..] {
            // Set event type buffer to value. After parsing as utf8.
            self.event_type.replace(String::from(value));
        } else if name == &b"data"[..] {
            match self.data {
                Some(ref mut data) => {
                    data.reserve(value.len() + 1);
                    data.push_str(value);
                    data.push('\n');
                }

                None => {
                    let mut s = String::with_capacity(value.len() + 1);
                    s.push_str(value);
                    s.push('\n');
                    self.data = Some(s);
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

    pub fn build_and_clear(&mut self) -> Result<crate::Event, Error> {
        Ok(crate::Event {
            event_type: self.event_type.take().unwrap_or_else(String::new),
            data: self.data.take().unwrap_or_else(String::new),
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

        let line = self.parse_line()?;

        if line.is_empty() {
            return Some(self.builder.build_and_clear());
        }

        // Check if there's a colon in the line
        match memchr(COLON, &line) {
            Some(0) => {
                return None;
            }

            Some(i) => {
                // name is all the characters to the left of the colon.
                let name = &line[0..i];

                // Let value be all the chars AFTER the colon.
                let value = &line[i + 1..];

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
        };

        None
    }

    fn parse_line(&mut self) -> Option<Bytes> {
        match memchr2(CR, LF, &self.buf) {
            Some(i) => {
                let line = self.buf.split_to(i).freeze();
                self.buf.advance(1);

                if !self.buf.is_empty() && self.buf[0] == LF {
                    self.buf.advance(1);
                }

                Some(line)
            }

            None => None,
        }
    }

    #[cfg(test)]
    fn bytes(&self) -> &[u8] {
        &self.buf
    }
}

impl From<&[u8]> for Parser {
    fn from(b: &[u8]) -> Self {
        let mut buf = BytesMut::with_capacity(b.remaining());
        buf.extend_from_slice(b);

        Self {
            buf,
            builder: EventBuilder::default(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn buf_cleared_line_ending_with_crlf() {
        let mut p = Parser::from(&b"\r\n"[..]);
        p.next();
        assert_eq!(p.bytes(), &[]);
    }

    #[test]
    fn buf_cleared_line_ending_with_cr() {
        let mut p = Parser::from(&b"\r"[..]);
        p.next();
        assert_eq!(p.bytes(), &[]);
    }

    #[test]
    fn buf_cleared_line_ending_with_lf() {
        let mut p = Parser::from(&b"\n"[..]);
        p.next();
        assert_eq!(p.bytes(), &[]);
    }

    #[test]
    fn lines_starting_with_colon_are_ignored() {
        let mut p = Parser::default();
        p.put(&b":ok"[..]);
        assert!(p.next().is_none());
    }

    #[test]
    fn test_memchr_order() {
        let bs = &b"abcd\r\n"[..];
        assert_eq!(memchr2(CR, LF, bs), Some(4));
        assert_eq!(memchr2(LF, CR, bs), Some(4));
    }
}
