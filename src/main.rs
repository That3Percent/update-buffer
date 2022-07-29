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
        thread::{self, spawn},
        time::{Duration, Instant},
    };

    use crate::{pair, reader_writer_pair};

    fn sleep() {
        // Sleeping 0ns to yield is not possible because the implementation
        // has a `while time > 0`.
        std::thread::sleep(Duration::from_nanos(1));
    }

    #[test]
    fn spew() {
        let (reader, mut writer) = reader_writer_pair(0u64, 0);
        let (sender, mut receiver) = pair::<u64>();

        let mut inner_reader = reader.clone();
        let threads = 7;
        let secs = 4;

        let end = Instant::now() + Duration::from_secs(secs);
        let finished = move || Instant::now() > end;

        let outer = thread::spawn(move || {
            let mut max_draw = 0;

            while !finished() {
                let processed = receiver.read(|items| {
                    let items = &*items;
                    let count = items.len();
                    if finished() {
                        return 0;
                    }
                    if count > 0 {
                        writer.update(|state| {
                            for value in items {
                                *state += *value;
                            }
                        });
                    }
                    count
                });
                max_draw = max_draw.max(processed);
                if processed == 0 {
                    sleep()
                }
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
            spawn(move || {
                while !finished() {
                    let hold = reader.latest();
                    let latest = *hold.deref();
                    assert!(latest >= prev);
                    prev = latest;
                    drop(hold);
                    if let Ok(count) = sender.write(1) {
                        if count > 3000 {
                            sleep();
                        }
                    }
                }
            });
        }
        drop(sender);

        outer.join().unwrap();
    }
}
