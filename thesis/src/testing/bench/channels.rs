use std::sync::{Arc, Barrier};

use test::Bencher;

const NUM_MESSAGES: usize = 2 << 13;

#[bench]
fn kanai_spsc(b: &mut Bencher) {
    let (input, output) = kanal::bounded(NUM_MESSAGES);
    let start_gate_0 = Arc::new(Barrier::new(2));
    let start_gate_1 = start_gate_0.clone();
    std::thread::spawn(move || loop {
        start_gate_0.wait();
        for _ in 0..NUM_MESSAGES {
            input.send(0u64).unwrap();
        }
    });
    b.iter(move || {
        start_gate_1.wait();
        for _ in 0..NUM_MESSAGES {
            output.recv().unwrap();
        }
    })
}

#[bench]
fn kanai_async_spsc(b: &mut Bencher) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(3)
        .build()
        .unwrap();
    b.iter(|| {
        rt.block_on(async {
            let (input, output) = kanal::bounded_async(NUM_MESSAGES);
            let producer = async {
                for _ in 0..NUM_MESSAGES {
                    input.send(0u64).await.unwrap();
                }
            };
            let consumer = async {
                for _ in 0..NUM_MESSAGES {
                    output.recv().await.unwrap();
                }
            };
            tokio::join!(producer, consumer);
        });
    })
}

#[bench]
fn crossbeam_spsc(b: &mut Bencher) {
    let (input, output) = crossbeam_channel::bounded(NUM_MESSAGES);
    let start_gate_0 = Arc::new(Barrier::new(2));
    let start_gate_1 = start_gate_0.clone();
    std::thread::spawn(move || loop {
        start_gate_0.wait();
        for _ in 0..NUM_MESSAGES {
            input.send(0u64).unwrap();
        }
    });
    b.iter(move || {
        start_gate_1.wait();
        for _ in 0..NUM_MESSAGES {
            output.recv().unwrap();
        }
    })
}

pub fn write_n_messages_to_cueue_buffer(buf: &mut [u64]) -> usize {
    let buf_len = buf.len().min(NUM_MESSAGES / 100);
    buf[0..buf_len].fill(0u64);
    buf_len
}

#[bench]
fn cueue_spsc(b: &mut Bencher) {
    let (input, mut output) = cueue::cueue(NUM_MESSAGES).unwrap();
    let start_gate_0 = Arc::new(Barrier::new(2));
    let start_gate_1 = start_gate_0.clone();
    std::thread::spawn(move || {
        let mut input = input;
        loop {
            start_gate_0.wait();
            let mut count = 0;
            while count < NUM_MESSAGES {
                let buf = input.write_chunk();
                let written = write_n_messages_to_cueue_buffer(buf);
                input.commit(written);
                count += written;
            }
        }
    });

    b.iter(move || {
        start_gate_1.wait();
        let mut count = 0;
        while count < NUM_MESSAGES {
            let read_result = output.read_chunk();
            count += read_result.len();
            output.commit();
        }
    });
}

pub fn write_n_messages_to_rtrb_buffer(mut buf: rtrb::chunks::WriteChunk<u64>) {
    let (first, second) = buf.as_mut_slices();
    first.fill(0);
    second.fill(0);
    buf.commit_all();
}

#[bench]
fn rtrb_spsc(b: &mut Bencher) {
    let (input, mut output) = rtrb::RingBuffer::new(NUM_MESSAGES);
    let start_gate_0 = Arc::new(Barrier::new(2));
    let start_gate_1 = start_gate_0.clone();
    std::thread::spawn(move || {
        let mut input = input;
        loop {
            start_gate_0.wait();
            let mut count = 0;
            while count < NUM_MESSAGES {
                let to_write = input.slots();
                let buf = input.write_chunk(to_write).unwrap();
                write_n_messages_to_rtrb_buffer(buf);
                count += to_write;
            }
        }
    });

    b.iter(move || {
        start_gate_1.wait();
        let mut count = 0;
        while count < NUM_MESSAGES {
            let to_read = output.slots();
            let read_result = output.read_chunk(to_read).unwrap();
            count += to_read;
            read_result.commit_all();
        }
    });
}
