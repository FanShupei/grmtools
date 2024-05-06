use num_traits::{PrimInt, Unsigned};

mod private {
    pub trait Priv {}

    impl Priv for u8 {}
    impl Priv for u16 {}
    impl Priv for u32 {}
    impl Priv for usize {}
}

pub trait Storage:
    'static + std::fmt::Debug + std::hash::Hash + PrimInt + Unsigned + private::Priv
{
    fn from_as_(x: usize) -> Self;
}

impl Storage for u8 {
    fn from_as_(x: usize) -> Self {
        x as u8
    }
}

impl Storage for u16 {
    fn from_as_(x: usize) -> Self {
        x as u16
    }
}

impl Storage for u32 {
    fn from_as_(x: usize) -> Self {
        x as u32
    }
}

impl Storage for usize {
    fn from_as_(x: usize) -> Self {
        x
    }
}
