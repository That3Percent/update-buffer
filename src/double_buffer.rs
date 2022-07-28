use parking_lot::RwLock;
use std::{ops::Deref, sync::Arc};

struct DoubleBufferData<T> {
    values: [RwLock<T>; 2],
}

/// a & b must be equal.
/// The API takes two values and does not verify equality to avoid requiring implementing
/// traits such as [Clone, Eq, Default]
pub fn reader_writer_pair<T>(a: T, b: T) -> (DoubleBufferReader<T>, DoubleBufferWriter<T>) {
    let data = Arc::new(DoubleBufferData {
        values: [RwLock::new(a), RwLock::new(b)],
    });
    (
        DoubleBufferReader {
            index: 0,
            data: data.clone(),
        },
        DoubleBufferWriter { data },
    )
}

#[derive(Clone)]
pub struct DoubleBufferReader<T> {
    index: usize,
    data: Arc<DoubleBufferData<T>>,
}

pub struct DoubleBufferWriter<T> {
    data: Arc<DoubleBufferData<T>>,
}

impl<T> DoubleBufferWriter<T> {
    /// f must be pure (though, it may hold data).
    /// The function will be called twice, once to update
    /// each snapshot.
    pub fn update<F>(&mut self, f: F)
    where
        for<'a> F: Fn(&'a mut T),
    {
        for value in &self.data.values {
            let mut write = value.write();
            f(&mut write);
        }
    }
}

impl<T> DoubleBufferReader<T> {
    pub fn latest<'a>(&'a mut self) -> impl 'a + Deref<Target = T> {
        loop {
            // This is guaranteed to only move forward in time,
            // and is almost guaranteed to acquire the lock "immediately".
            // These guarantees come from the invariant that there is
            // a single writer and it can only be in a few possible states.
            match self.data.values[self.index].try_read() {
                Some(data) => return data,
                None => self.index = if self.index == 0 { 1 } else { 0 },
            }
        }
    }
}
