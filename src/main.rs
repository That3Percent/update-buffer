mod double_buffer;

pub use double_buffer::{reader_writer_pair, DoubleBufferReader, DoubleBufferWriter};

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use num_format::{Locale, ToFormattedString};
    use std::{
        ops::Deref,
        sync::{
            atomic::{AtomicUsize, Ordering::SeqCst},
            mpsc::channel,
            Arc,
        },
        thread::{self, spawn},
        time::{Duration, Instant},
    };

    use crate::reader_writer_pair;

    fn sleep() {
        // Sleeping 0ns to yield is not possible because the implementation
        // has a while time > 0.
        std::thread::sleep(Duration::from_nanos(1));
    }

    #[test]
    fn spew() {
        let buffer_count = Arc::new(AtomicUsize::new(0));
        let (reader, mut writer) = reader_writer_pair(0u64, 0);
        let (sender, receiver) = channel::<u64>();
        let mut handles = Vec::new();

        let mut inner_reader = reader.clone();
        let inner_buffer_count = buffer_count.clone();
        let threads = 7;

        let secs = 20;
        let end = Instant::now() + Duration::from_secs(secs);

        let outer = thread::spawn(move || {
            // TODO: Potential problem is that this only pulls updates.
            // We do need to force
            let mut buffer = Vec::new();
            let mut max_draw = 0;

            while Instant::now() < end {
                // TODO: The cap here should be however many events had been
                // pushed since last time on the previous iteration.
                let draw = inner_buffer_count.load(SeqCst);
                max_draw = draw.max(max_draw);
                for _ in 0..draw {
                    if Instant::now() > end {
                        break;
                    }

                    if let Ok(next) = receiver.recv() {
                        buffer.push(next);
                    }
                }
                if buffer.len() > 0 {
                    writer.update(|state| {
                        for value in buffer.iter() {
                            *state += value;
                        }
                    });
                    inner_buffer_count.fetch_sub(buffer.len(), SeqCst);

                    buffer.clear();
                } else {
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
            let sender = sender.clone();
            let mut reader = reader.clone();
            let buffer_count = buffer_count.clone();
            let mut prev = 0;
            let handle = spawn(move || {
                while Instant::now() < end {
                    // Hold the lock even while sleeping
                    let hold = reader.latest();
                    let latest = *hold.deref();
                    assert!(latest >= prev);
                    prev = latest;
                    drop(hold);
                    let _ignore = sender.send(1);
                    if buffer_count.fetch_add(1, SeqCst) > 3000 {
                        sleep();
                    }
                }
            });
            handles.push(handle);
        }
        drop(sender);

        for handle in handles.drain(..) {
            handle.join().unwrap();
        }

        outer.join().unwrap();
    }
}
