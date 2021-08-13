use bytes::Bytes;
use futures_core::Stream;

use std::{
    error::Error as StdError,
    pin::Pin,
    task::{Context, Poll},
};

mod error;
mod event;
mod parser;

pub use {
    error::{Error, ErrorKind},
    event::Event,
};

use parser::Parser;

struct Body<S> {
    inner: S,

    parser: Parser,
}

impl<S, E> Stream for Body<S>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: StdError + Unpin,
{
    type Item = Result<Event, Error>;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        // Whenever the parser cannot yet produce an Event. We want to poll the underlying
        // Stream.
        //
        // However, if we ready Ready(Some(Ok(bs))) from inner stream we also want to parse.
        //
        // This is probably not the nicest code, but for now, let's always start by
        // trying to parse.

        match self.parser.next() {
            Some(Ok(ev)) => return Poll::Ready(Some(Ok(ev))),
            Some(Err(err)) => return Poll::Ready(Some(Err(Error::parser(err)))),
            None => (),
        }

        match Pin::new(&mut self.inner).poll_next(ctx) {
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(Error::inner(err)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(bs))) => {
                self.parser.put(bs);
                Poll::Ready(None)
            }
        }
    }
}

impl<S, E> From<S> for Body<S>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: StdError,
{
    fn from(inner: S) -> Self {
        Self {
            inner,
            parser: Parser::default(),
        }
    }
}
