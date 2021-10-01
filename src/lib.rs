use {bytes::Bytes, futures_core::Stream};

mod body;
mod error;
mod event;
mod parser;

pub use {
    body::Body,
    error::{Error, ErrorKind},
    event::Event,
};

pub trait Sse<S> {
    fn into_sse(self) -> Body<S>;
}

impl<S, E> Sse<S> for S
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::error::Error,
{
    fn into_sse(self) -> Body<S> {
        Body::from(self)
    }
}
