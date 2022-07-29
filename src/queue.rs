// TODO: Make `read` async (or have async version)
// TODO: Make `update` async (or have version)

use {
    std::sync::{
        atomic::{AtomicIsize, Ordering::SeqCst},
        Arc,
    },
    tokio::sync::mpsc::{error::SendError, unbounded_channel, UnboundedReceiver, UnboundedSender},
};

pub struct QueueReader<T> {
    count: Arc<AtomicIsize>,
    recv: UnboundedReceiver<T>,
}

#[derive(Clone)]
pub struct QueueWriter<T> {
    count: Arc<AtomicIsize>,
    send: UnboundedSender<T>,
}

impl<T> QueueWriter<T> {
    pub fn write(&mut self, value: T) -> Result<isize, SendError<T>> {
        match self.send.send(value) {
            Ok(()) => Ok(self.count.fetch_add(1, SeqCst) + 1),
            Err(e) => Err(e),
        }
    }
}

impl<T> QueueReader<T> {
    pub async fn read<F, R>(&mut self, f: F) -> R
    where
        for<'a> F: FnOnce(&'a mut [T]) -> R,
    {
        // Reads `count` items from `source`.
        struct Iter<'a, I> {
            count: isize,
            source: &'a mut UnboundedReceiver<I>,
            first: Option<I>,
        }

        impl<T> Iterator for Iter<'_, T> {
            type Item = T;
            fn next(&mut self) -> Option<Self::Item> {
                if let Some(first) = self.first.take() {
                    self.count -= 1;
                    return Some(first);
                }
                if self.count < 1 {
                    return None;
                }
                self.count -= 1;
                self.source.try_recv().ok()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.count as usize, Some(self.count as usize))
            }
        }

        let iter = Iter {
            count: self.count.load(SeqCst).max(1),
            first: self.recv.recv().await,
            source: &mut self.recv,
        };

        second_stack::buffer(iter, |items| {
            self.count.fetch_sub(items.len() as isize, SeqCst);
            f(items)
        })
    }
}

pub fn pair<T>() -> (QueueWriter<T>, QueueReader<T>) {
    let (send, recv) = unbounded_channel();
    let count = Arc::new(AtomicIsize::new(0));
    (
        QueueWriter {
            send,
            count: count.clone(),
        },
        QueueReader { recv, count },
    )
}
