use std::{error::Error as StdError, fmt};

#[derive(Debug, Clone)]
pub struct Error<E> {
    kind: Box<ErrorKind<E>>,
}

impl<E> Error<E> {
    pub fn kind(self) -> ErrorKind<E> {
        *self.kind
    }
}

impl<E> fmt::Display for Error<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind.as_ref() {
            ErrorKind::Inner(ref err) => write!(f, "Transport error: {}", err),
            ErrorKind::Sse(err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl<E> StdError for Error<E>
where
    E: StdError,
{
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self.kind.as_ref() {
            ErrorKind::Inner(ref err) => err.source(),
            ErrorKind::Sse(err) => err.source(),
        }
    }
}

impl<E> Error<E> {
    pub(crate) fn inner(err: E) -> Self {
        Self {
            kind: Box::new(ErrorKind::Inner(err)),
        }
    }

    pub(crate) fn parser(err: crate::parser::Error) -> Self {
        Self {
            kind: Box::new(ErrorKind::Sse(err)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ErrorKind<E> {
    Sse(crate::parser::Error),
    Inner(E),
}
