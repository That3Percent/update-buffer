// TODO: Make `read` async (or have async version)
// TODO: Make `update` async (or have version)

use std::sync::{
    atomic::{AtomicUsize, Ordering::SeqCst},
    mpsc::{channel, Receiver, SendError, Sender},
    Arc,
};

pub struct QueueReader<T> {
    count: Arc<AtomicUsize>,
    recv: Receiver<T>,
}

#[derive(Clone)]
pub struct QueueWriter<T> {
    count: Arc<AtomicUsize>,
    send: Sender<T>,
}

impl<T> QueueWriter<T> {
    pub fn write(&mut self, value: T) -> Result<usize, SendError<T>> {
        match self.send.send(value) {
            Ok(()) => Ok(self.count.fetch_add(1, SeqCst) + 1),
            Err(e) => Err(e),
        }
    }
}

impl<T> QueueReader<T> {
    pub fn read<F, R>(&mut self, f: F) -> R
    where
        for<'a> F: FnOnce(&'a mut [T]) -> R,
    {
        // Reads `count` items from `source`.
        struct Iter<'a, I> {
            count: usize,
            source: &'a mut Receiver<I>,
        }

        impl<T> Iterator for Iter<'_, T> {
            type Item = T;
            fn next(&mut self) -> Option<Self::Item> {
                if self.count == 0 {
                    return None;
                }
                self.count -= 1;
                self.source.recv().ok()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.count, Some(self.count))
            }
        }

        let iter = Iter {
            count: self.count.swap(0, SeqCst),
            source: &mut self.recv,
        };

        second_stack::buffer(iter, f)
    }
}

pub fn pair<T>() -> (QueueWriter<T>, QueueReader<T>) {
    let (send, recv) = channel();
    let count = Arc::new(AtomicUsize::new(0));
    (
        QueueWriter {
            send,
            count: count.clone(),
        },
        QueueReader { recv, count },
    )
}
