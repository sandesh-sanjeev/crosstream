# Crosstream

Crosstream provides different types of ring buffers, along with primitives to build them yourself.

[![Build](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml)

[![Coverage Status](https://coveralls.io/repos/github/sandesh-sanjeev/crosstream/badge.svg?branch=master)](https://coveralls.io/github/sandesh-sanjeev/crosstream?branch=master)

## Security

Crate makes some use of unsafe for better perf. However we use unsafe
only where it is trivially provable correct to human readers and proof engines. 

## Tests

```bash
# No features
$ cargo test

# Tests with all features for coverage
$ cargo tarpaulin
```

## Miri

Note that this takes quite some time to finish execution.

```bash
# Install Miri on nightly rust
$ rustup +nightly component add miri

# Override workspace to nightly
$ rustup override set nightly

# Run miri on tests
$ MIRIFLAGS=-Zmiri-disable-isolation cargo miri test

# Remove workspace override.
$ rustup override remove
```

## Benchmarks

On my Apple M1 Pro with 32 GB Memory.

* Size of record is 8 bytes (u64).
* About 8GB of total space for ring buffer. Holds just over a billion records.
* Obviously means nothing unless you test it yourself, for your use case.
* Comparison with another popular [ring buffer](https://docs.rs/ringbuffer/latest/ringbuffer/struct.AllocRingBuffer.htm) crate with similar features.

```bash
$ cargo bench
hadron/append_from_slice/2048
                        time:   [451.31 ns 475.34 ns 498.59 ns]
                        thrpt:  [32.861 GB/s 34.468 GB/s 36.303 GB/s]

hadron/append_from_slice/5120
                        time:   [1.1388 µs 1.1942 µs 1.2500 µs]
                        thrpt:  [32.769 GB/s 34.299 GB/s 35.967 GB/s]

hadron/append_from_slice/10240
                        time:   [2.1686 µs 2.3038 µs 2.4395 µs]
                        thrpt:  [33.581 GB/s 35.559 GB/s 37.776 GB/s]

oracle/append_from_slice/2048
                        time:   [3.8836 µs 3.9204 µs 3.9545 µs]
                        thrpt:  [4.1431 GB/s 4.1791 GB/s 4.2188 GB/s]

oracle/append_from_slice/5120
                        time:   [9.7892 µs 9.8126 µs 9.8339 µs]
                        thrpt:  [4.1652 GB/s 4.1742 GB/s 4.1842 GB/s]

oracle/append_from_slice/10240
                        time:   [18.188 µs 18.341 µs 18.496 µs]
                        thrpt:  [4.4292 GB/s 4.4665 GB/s 4.5041 GB/s]

```

### Profiler

```bash
# Install cargo flamegraph.
$ cargo install flamegraph

# Run benchmarks with profiler.
$ cargo flamegraph --bench hadron -- --bench --profile-time 60
```