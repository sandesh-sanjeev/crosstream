//! Definition of benchmarks.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use crosstream::{Hadron, Item};
use ringbuffer::AllocRingBuffer;
use std::time::Duration;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

// About 8 GB of memory for benchmarks.
const CAPACITY: usize = 1_073_741_824;

// Base batch size, different batch sizes will be multiples of this number.
const BATCH_SIZE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
struct Log(u64);

impl Item for Log {
    const SIZE: usize = size_of::<Self>();

    fn to_bytes_slice(records: &[Self]) -> &[u8] {
        records.as_bytes()
    }

    fn from_byte_slice(bytes: &[u8]) -> &[Self] {
        <[Self]>::ref_from_bytes(bytes).expect("Should transmute back to items")
    }
}

/// A reference implementation of ring buffer from a popular crate.
struct Oracle<T>(AllocRingBuffer<T>);

impl<T: Item> Oracle<T> {
    fn with_capacity(capacity: usize) -> Self {
        Self(AllocRingBuffer::new(capacity))
    }

    fn append_from_slice(&mut self, items: &[T]) {
        self.0.extend(items.iter().map(|item| *item));
    }
}

criterion_main!(benches);
criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(60));
    targets = hadron_bench, oracle_bench
}

macro_rules! run_bench {
    ($name:ident, $ring:ty, $id:literal) => {
        fn $name(c: &mut Criterion) {
            // Benchmark group for the benchmark.
            // This captures the different ring buffer implementations.
            let mut group = c.benchmark_group($id);

            // Create a ring buffer.
            let mut ring = <$ring>::with_capacity(CAPACITY);

            // Pre-populate the ring buffer with records.
            // This makes sure page faults don't occur during benchmarks.
            let items: Vec<_> = (0..=BATCH_SIZE as u64).map(Log).collect();
            for _ in (0..(CAPACITY / BATCH_SIZE)) {
                ring.append_from_slice(&items);
            }

            // Run tests with different batch sizes.
            for i in [2, 5, 10] {
                // Batch size for the test.
                let batch_size = BATCH_SIZE * i;

                // Tests to batch append into the ring buffer.
                let items: Vec<_> = (1..=batch_size as u64).map(Log).collect();
                group.throughput(Throughput::BytesDecimal((batch_size * Log::SIZE) as _));
                group.bench_function(format!("append_from_slice/{batch_size}"), |bencher| {
                    bencher.iter(|| ring.append_from_slice(&items))
                });
            }

            group.finish();
        }
    };
}

run_bench!(hadron_bench, Hadron<Log>, "hadron");
run_bench!(oracle_bench, Oracle<Log>, "oracle");
