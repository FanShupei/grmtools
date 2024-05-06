use std::{error::Error, fmt, hash::Hash};

use cfgrammar::{Span, Storage};
use lrpar::{Lexeme, LexerTypes};

use crate::LRLexError;

/// lrlex's standard [LexerTypes] `struct`, provided as a convenience.
#[derive(Debug)]
pub struct DefaultLexerTypes<T: Storage = u32> {
    phantom: std::marker::PhantomData<T>,
}

impl<T: Storage> LexerTypes for DefaultLexerTypes<T> {
    type LexemeT = DefaultLexeme<T>;
    type StorageT = T;
    type LexErrorT = LRLexError;
}

/// lrlex's standard lexeme struct, provided as a convenience.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DefaultLexeme<StorageT = u32> {
    start: usize,
    len: usize,
    faulty: bool,
    tok_id: StorageT,
}

impl<StorageT: Storage> Lexeme<StorageT> for DefaultLexeme<StorageT> {
    fn new(tok_id: StorageT, start: usize, len: usize) -> Self {
        DefaultLexeme {
            start,
            len,
            faulty: false,
            tok_id,
        }
    }

    fn new_faulty(tok_id: StorageT, start: usize, len: usize) -> Self {
        DefaultLexeme {
            start,
            len,
            faulty: true,
            tok_id,
        }
    }

    fn tok_id(&self) -> StorageT {
        self.tok_id
    }

    fn span(&self) -> Span {
        Span::new(self.start, self.start + self.len)
    }

    fn faulty(&self) -> bool {
        self.faulty
    }
}

impl<StorageT: Storage> fmt::Display for DefaultLexeme<StorageT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DefaultLexeme[{}..{}]",
            self.span().start(),
            self.span().end()
        )
    }
}

impl<StorageT: Storage> Error for DefaultLexeme<StorageT> {}
