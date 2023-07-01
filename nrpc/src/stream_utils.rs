use futures::Stream;

use core::{pin::Pin, task::{Context, Poll}};
use core::marker::{PhantomData, Unpin};

#[derive(Default, Clone, Copy)]
pub struct EmptyStream<T> {
    _idc: PhantomData<T>,
}

impl <T> Stream for EmptyStream<T> {
    type Item = T;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

#[derive(Clone)]
pub struct OnceStream<T: Unpin> {
    item: Option<T>,
}

impl <T: Unpin> OnceStream<T> {
    pub fn once(item: T) -> Self {
        Self { item: Some(item) }
    }
}

impl <T: Unpin> Stream for OnceStream<T> {
    type Item = T;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.item.take())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.item.is_some() {
            (1, Some(1))
        } else {
            (0, Some(0))
        }
    }
}

#[derive(Clone)]
pub struct VecStream<T: Unpin> {
    items: std::collections::VecDeque<T>,
}

impl <T: Unpin> VecStream<T> {
    pub fn from_iter(iter: impl Iterator<Item=T>) -> Self {
        Self { items: iter.collect() }
    }
}

impl <T: Unpin> Stream for VecStream<T> {
    type Item = T;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.items.pop_front())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.items.len(), Some(self.items.len()))
    }
}
