#![allow(clippy::cognitive_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![forbid(unsafe_code)]
#![deny(unreachable_pub)]

use std::{hash::Hash, mem::size_of};

use num_traits::{PrimInt, Unsigned};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

mod itemset;
mod pager;
mod stategraph;
pub mod statetable;

pub use crate::{
    stategraph::StateGraph,
    statetable::{Action, StateTable, StateTableError, StateTableErrorKind},
};
use cfgrammar::{yacc::YaccGrammar, Storage};

macro_rules! IdxNewtype {
    ($(#[$attr:meta])* $n: ident) => {
        $(#[$attr])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(feature="serde", derive(Serialize, Deserialize))]
        pub struct $n<T>(pub T);

        impl<T: PrimInt + Unsigned> From<$n<T>> for usize {
            fn from($n(st): $n<T>) -> Self {
                debug_assert!(size_of::<usize>() >= size_of::<T>());
                num_traits::cast(st).unwrap()
            }
        }

        impl<T: PrimInt + Unsigned> From<$n<T>> for u32 {
            fn from($n(st): $n<T>) -> Self {
                debug_assert!(size_of::<u32>() >= size_of::<T>());
                num_traits::cast(st).unwrap()
            }
        }

        impl<T: PrimInt + Unsigned> $n<T> {
            pub fn as_storaget(&self) -> T {
                let $n(st) = self;
                *st
            }
        }

        impl<T: Storage> $n<T> {
            pub fn from_usize(x: usize) -> Self {
                $n(T::from_as_(x))
            }
        }
    }
}

IdxNewtype!(
    /// A type specifically for state table indices.
    ///
    /// It is guaranteed that `StIdx` can be converted, without loss of precision, to `usize` with
    /// the idiom `usize::from(...)`.
    StIdx
);

#[derive(Clone, Copy)]
pub enum Minimiser {
    Pager,
}

pub fn from_yacc<StorageT: Storage>(
    grm: &YaccGrammar<StorageT>,
    m: Minimiser,
) -> Result<(StateGraph<StorageT>, StateTable<StorageT>), StateTableError<StorageT>> {
    match m {
        Minimiser::Pager => {
            let sg = pager::pager_stategraph(grm);
            let st = StateTable::new(grm, &sg)?;
            Ok((sg, st))
        }
    }
}
