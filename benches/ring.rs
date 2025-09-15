use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use crosstream::{
    OffHeapSeqRing, OnHeapSeqRing, QueryBuf, Record, SeqRecord, SeqRing, Storage, VecSeqRing,
};
use std::{cell::Cell, time::Duration};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

// About 8 GB of memory for benchmarks.
const SLOTS: usize = 64;
const SLOT_CAPACITY: usize = 16_777_216;

// Number of records to append/query from ring.
const BATCH_SIZE: usize = 1024 * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
struct Log(u64);

impl SeqRecord for Log {
    fn seq_no(&self) -> u64 {
        self.0
    }
}

criterion_main!(benches);
criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(15));
    targets = vec_bench, on_heap_bench, off_heap_bench
}

fn vec_bench(c: &mut Criterion) {
    run_bench(c, VecSeqRing::new(SLOT_CAPACITY, SLOTS, 0), "Vec");
}

fn on_heap_bench(c: &mut Criterion) {
    run_bench(c, OnHeapSeqRing::new(SLOT_CAPACITY, SLOTS, 0), "OnHeap");
}

fn off_heap_bench(c: &mut Criterion) {
    run_bench(c, OffHeapSeqRing::new(SLOT_CAPACITY, SLOTS, 0), "OffHeap");
}

fn run_bench<S: Storage<Record = Log>>(c: &mut Criterion, mut ring: SeqRing<S>, name: &str) {
    // Create a ring buffer.
    let prev_seq_no = Cell::new(0);

    // Append tests.
    let mut group = c.benchmark_group(name);
    group.throughput(Throughput::BytesDecimal((BATCH_SIZE * Log::size()) as _));
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
                ring.append(&records).expect("Should append records");
            },
            // Large input to help with memory usage.
            BatchSize::LargeInput,
        )
    });
    group.finish();

    // Query tests.
    let query_seq_no = Cell::new(0);
    let mut buf = QueryBuf::new(BATCH_SIZE);
    let mut group = c.benchmark_group(name);
    group.throughput(Throughput::BytesDecimal((BATCH_SIZE * Log::size()) as _));
    group.bench_function("query", |bencher| {
        bencher.iter(|| {
            // Query for next batch of records.
            ring.query_after(query_seq_no.get(), &mut buf);

            // Wrap around if last record in ring buffer is reached.
            if let Some(log) = buf.records().last() {
                if log.seq_no() == prev_seq_no.get() {
                    query_seq_no.set(0);
                } else {
                    query_seq_no.set(log.seq_no());
                }
            }
        })
    });
    group.finish();
}
