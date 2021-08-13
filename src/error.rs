use std::error::Error as StdError;

#[derive(Debug, Clone)]
pub struct Error {
    kind: Box<ErrorKind>,
}

impl Error {
    pub(crate) fn inner<E>(_err: E) -> Self
    where
        E: StdError,
    {
        Self {
            kind: Box::new(ErrorKind::Inner),
        }
    }

    pub(crate) fn parser(_err: crate::parser::Error) -> Self {
        Self {
            kind: Box::new(ErrorKind::Sse),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Sse,
    Inner,
}
