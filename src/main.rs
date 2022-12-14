mod double_buffer;
mod queue;

pub use double_buffer::{reader_writer_pair, DoubleBufferReader, DoubleBufferWriter};
pub use queue::{pair, QueueReader, QueueWriter};

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use num_format::{Locale, ToFormattedString};
    use std::{
        ops::Deref,
        time::{Duration, Instant},
    };
    use tokio::{spawn, test};

    use crate::{pair, reader_writer_pair};

    #[test]
    async fn spew() {
        let (reader, mut writer) = reader_writer_pair(0u64, 0);
        let (sender, mut receiver) = pair::<u64>();

        let mut inner_reader = reader.clone();
        let threads = 1000;
        let secs = 10;
        let end = Instant::now() + Duration::from_secs(secs);
        let finished = move || Instant::now() > end;

        let outer = spawn(async move {
            let mut max_draw = 0;
            let mut buffer = Vec::new();

            while !finished() {
                receiver.read(&mut buffer).await;
                if Instant::now() < end {
                    writer
                        .update(|state| {
                            for value in &buffer[..] {
                                *state += *value;
                            }
                        })
                        .await;
                }
                max_draw = max_draw.max(buffer.len());
                buffer.clear();
            }
            let latest = *inner_reader.latest().deref();
            println!(
                "Rate: {}/s",
                (latest / secs).to_formatted_string(&Locale::en)
            );
            println!("Largest buffer: {}", max_draw);
        });

        for _ in 0..threads {
            let mut sender = sender.clone();
            let mut reader = reader.clone();
            let mut prev = 0;
            spawn(async move {
                while !finished() {
                    {
                        let hold = reader.latest();
                        let latest = *hold.deref();
                        assert!(latest >= prev);
                        prev = latest;
                    }
                    let _ = sender.write(1);
                    // Make request. If this task doesn't await at some point
                    // then the writer task is never scheduled and becomes starved.
                    tokio::task::yield_now().await;
                }
            });
        }
        outer.await.unwrap();
    }
}
