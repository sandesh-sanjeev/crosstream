//! Definition of benchmarks.

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use crosstream::{Hadron, Item};
use ringbuffer::AllocRingBuffer;
use std::{cell::Cell, time::Duration};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

// About 4 GB of memory for benchmarks.
const CAPACITY: usize = 536_870_912;

// Number of records to append/query from ring.
const BATCH_SIZE: usize = 1024 * 2;

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
            // Create a ring buffer.
            let mut ring = <$ring>::with_capacity(CAPACITY);

            // Define the append tests.
            let mut group = c.benchmark_group($id);
            group.throughput(Throughput::BytesDecimal((BATCH_SIZE * Log::SIZE) as _));

            // Run the benchmark.
            let prev_seq_no = Cell::new(0);
            group.bench_function("append", |bencher| {
                bencher.iter_batched(
                    || {
                        // Range of records to create.
                        let start_seq_no = prev_seq_no.get() + 1;
                        let end_seq_no = start_seq_no + BATCH_SIZE as u64;

                        // For next iteration.
                        prev_seq_no.set(end_seq_no);

                        // Records to insert into ring buffer.
                        (start_seq_no..=end_seq_no).map(Log).collect::<Vec<_>>()
                    },
                    |records| {
                        // Append records into the ring buffer.
                        ring.append_from_slice(&records);
                    },
                    // Large input to help with memory usage.
                    BatchSize::LargeInput,
                )
            });
            group.finish();
        }
    };
}

run_bench!(hadron_bench, Hadron<Log>, "hadron");
run_bench!(oracle_bench, Oracle<Log>, "oracle");
