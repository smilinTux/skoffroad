use std::marker::PhantomData;

// Minimal stub for State type
pub struct State<T> {
    _marker: PhantomData<T>,
}
