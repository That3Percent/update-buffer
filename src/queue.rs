// TODO: Make `read` async (or have async version)
// TODO: Make `update` async (or have version)

use std::future::Future;

use tokio::sync::mpsc::{error::SendError, unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct QueueReader<T> {
    buffer: Vec<T>,
    recv: UnboundedReceiver<T>,
}

#[derive(Clone)]
pub struct QueueWriter<T> {
    send: UnboundedSender<T>,
}

impl<T> QueueWriter<T> {
    pub fn write(&mut self, value: T) -> Result<(), SendError<T>> {
        match self.send.send(value) {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl<T> QueueReader<T> {
    pub async fn read<F, R, Fut>(&mut self, f: F) -> R
    where
        T: 'static,
        for<'a> F: FnOnce(&'a [T]) -> Fut + 'a,
        Fut: Future<Output = R>,
    {
        let mut buffer = Vec::new();
        if let Some(first) = self.recv.recv().await {
            buffer.push(first);
        }

        while let Ok(next) = self.recv.try_recv() {
            buffer.push(next);
        }

        let result = f(&buffer[..]).await;
        drop(buffer);
        //self.buffer.clear();
        result
    }
}

pub fn pair<T>() -> (QueueWriter<T>, QueueReader<T>) {
    let (send, recv) = unbounded_channel();
    (
        QueueWriter { send },
        QueueReader {
            recv,
            buffer: Vec::new(),
        },
    )
}
