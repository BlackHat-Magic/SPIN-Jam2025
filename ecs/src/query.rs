pub use crate::*;

pub trait QueryPattern {}

pub struct Query<T: QueryPattern> {
    _marker: std::marker::PhantomData<T>,
}
