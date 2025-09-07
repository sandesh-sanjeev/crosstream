//! Benchmarks for a [`Ring`].

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use crosstream_ring::{Record, Ring};
use rand::Rng;
use std::{hint::black_box, time::Duration};

const BATCH_SIZE: usize = 10240;
const CAPACITY: usize = 536_870_912;

criterion_main!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

criterion_group!(
    name = u8;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_u8, run_vec_benchmark_u8,
);

criterion_group!(
    name = u16;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_u16, run_vec_benchmark_u16,
);

criterion_group!(
    name = u32;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_u32, run_vec_benchmark_u32,
);

criterion_group!(
    name = u64;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_u64, run_vec_benchmark_u64,
);

criterion_group!(
    name = u128;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_u128, run_vec_benchmark_u128,
);

criterion_group!(
    name = i8;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_i8, run_vec_benchmark_i8,
);

criterion_group!(
    name = i16;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_i16, run_vec_benchmark_i16,
);

criterion_group!(
    name = i32;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_i32, run_vec_benchmark_i32,
);

criterion_group!(
    name = i64;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_i64, run_vec_benchmark_i64,
);

criterion_group!(
    name = i128;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_i128, run_vec_benchmark_i128,
);

criterion_group!(
    name = f32;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_f32, run_vec_benchmark_f32,
);

criterion_group!(
    name = f64;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))
        .measurement_time(Duration::from_secs(30));
    targets = run_ring_benchmark_f64, run_vec_benchmark_f64,
);

macro_rules! run_benchmark {
    ($name:ident, $type:ty, $num:ty, $group:literal) => {
        fn $name(c: &mut Criterion) {
            // Allocate capacity for a buffer.
            let mut rng = rand::rng();
            let mut buf = <$type>::with_capacity(CAPACITY);

            // Extend from slice.
            let name = concat!("push_", stringify!($num));
            let mut group = c.benchmark_group($group);
            group.throughput(Throughput::Elements(1));
            group.bench_function(name, |b| {
                b.iter_batched(
                    || rng.random::<$num>(),
                    |record| buf.bench_push(record),
                    BatchSize::SmallInput,
                )
            });
            group.finish();

            // Extend from slice.
            let name = concat!("extend_from_slice_", stringify!($num));
            let mut group = c.benchmark_group($group);
            group.throughput(Throughput::Elements(BATCH_SIZE as _));
            group.bench_function(name, |b| {
                b.iter_batched(
                    || rand::random_iter().take(BATCH_SIZE).collect::<Vec<$num>>(),
                    |records| buf.bench_extend_slice(&records),
                    BatchSize::LargeInput,
                )
            });
            group.finish();

            // Make sure buffer has full capacity.
            buf.clear();
            while (buf.capacity() - buf.len() >= BATCH_SIZE) {
                let records = rand::random_iter().take(BATCH_SIZE).collect::<Vec<$num>>();
                buf.extend_from_slice(&records);
            }

            // Iterate through all records.
            let name = concat!("iter_", stringify!($num));
            let mut group = c.benchmark_group($group);
            group.throughput(Throughput::Elements(CAPACITY as _));
            group.bench_function(name, |b| {
                b.iter_batched(
                    || rng.random::<$num>(),
                    |needle| black_box(buf.iter().any(|record| *record == needle)),
                    BatchSize::SmallInput,
                )
            });
            group.finish();
        }
    };
}

run_benchmark!(run_ring_benchmark_u8, Ring<u8>, u8, "Ring");
run_benchmark!(run_vec_benchmark_u8, Vec<u8>, u8, "Vec");

run_benchmark!(run_ring_benchmark_u16, Ring<u16>, u16, "Ring");
run_benchmark!(run_vec_benchmark_u16, Vec<u16>, u16, "Vec");

run_benchmark!(run_ring_benchmark_u32, Ring<u32>, u32, "Ring");
run_benchmark!(run_vec_benchmark_u32, Vec<u32>, u32, "Vec");

run_benchmark!(run_ring_benchmark_u64, Ring<u64>, u64, "Ring");
run_benchmark!(run_vec_benchmark_u64, Vec<u64>, u64, "Vec");

run_benchmark!(run_ring_benchmark_u128, Ring<u128>, u128, "Ring");
run_benchmark!(run_vec_benchmark_u128, Vec<u128>, u128, "Vec");

run_benchmark!(run_ring_benchmark_i8, Ring<i8>, i8, "Ring");
run_benchmark!(run_vec_benchmark_i8, Vec<i8>, i8, "Vec");

run_benchmark!(run_ring_benchmark_i16, Ring<i16>, i16, "Ring");
run_benchmark!(run_vec_benchmark_i16, Vec<i16>, i16, "Vec");

run_benchmark!(run_ring_benchmark_i32, Ring<i32>, i32, "Ring");
run_benchmark!(run_vec_benchmark_i32, Vec<i32>, i32, "Vec");

run_benchmark!(run_ring_benchmark_i64, Ring<i64>, i64, "Ring");
run_benchmark!(run_vec_benchmark_i64, Vec<i64>, i64, "Vec");

run_benchmark!(run_ring_benchmark_i128, Ring<i128>, i128, "Ring");
run_benchmark!(run_vec_benchmark_i128, Vec<i128>, i128, "Vec");

run_benchmark!(run_ring_benchmark_f32, Ring<f32>, f32, "Ring");
run_benchmark!(run_vec_benchmark_f32, Ring<f32>, f32, "Vec");

run_benchmark!(run_ring_benchmark_f64, Ring<f64>, f64, "Ring");
run_benchmark!(run_vec_benchmark_f64, Ring<f64>, f64, "Vec");

trait RingBuffer<T> {
    fn bench_push(&mut self, record: T);
    fn bench_extend_slice(&mut self, records: &[T]);
}

impl<T: Record> RingBuffer<T> for Ring<T> {
    fn bench_push(&mut self, record: T) {
        if self.is_full() {
            self.trim(CAPACITY / 4);
        }

        self.push(record);
    }

    fn bench_extend_slice(&mut self, records: &[T]) {
        let remaining = self.remaining();
        if remaining < records.len() {
            self.trim(CAPACITY / 4);
        }

        self.extend_from_slice(records);
    }
}

impl<T: Copy> RingBuffer<T> for Vec<T> {
    fn bench_push(&mut self, record: T) {
        if self.len() >= CAPACITY {
            self.drain(..(CAPACITY / 4));
        }

        self.push(record);
    }

    fn bench_extend_slice(&mut self, records: &[T]) {
        let remaining = CAPACITY - self.len();
        if remaining < records.len() {
            self.drain(..(CAPACITY / 4));
        }

        self.extend_from_slice(records);
    }
}
