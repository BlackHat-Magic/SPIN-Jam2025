pub use crate::ecs::*;

pub trait QueryPattern {}



pub struct Query<T: QueryPattern> {
    _marker: std::marker::PhantomData<T>,


}