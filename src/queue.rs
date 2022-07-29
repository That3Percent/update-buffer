use tokio::sync::mpsc::{error::SendError, unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct QueueReader<T> {
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
    pub async fn read(&mut self, buffer: &mut Vec<T>) {
        if let Some(first) = self.recv.recv().await {
            buffer.push(first);
        }

        while let Ok(next) = self.recv.try_recv() {
            buffer.push(next);
        }
    }
}

pub fn pair<T>() -> (QueueWriter<T>, QueueReader<T>) {
    let (send, recv) = unbounded_channel();
    (QueueWriter { send }, QueueReader { recv })
}
