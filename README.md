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
                        time:   [514.61 ns 531.95 ns 546.95 ns]
                        thrpt:  [29.955 GB/s 30.800 GB/s 31.838 GB/s]

hadron/append_from_slice/5120
                        time:   [1.3023 µs 1.3266 µs 1.3514 µs]
                        thrpt:  [30.309 GB/s 30.875 GB/s 31.452 GB/s]

hadron/append_from_slice/10240
                        time:   [2.8596 µs 2.9025 µs 2.9421 µs]
                        thrpt:  [27.844 GB/s 28.224 GB/s 28.648 GB/s]

oracle/append_from_slice/2048
                        time:   [3.5775 µs 3.6246 µs 3.6700 µs]
                        thrpt:  [4.4643 GB/s 4.5202 GB/s 4.5797 GB/s]

oracle/append_from_slice/5120
                        time:   [9.6138 µs 9.6474 µs 9.6791 µs]
                        thrpt:  [4.2318 GB/s 4.2457 GB/s 4.2605 GB/s]

oracle/append_from_slice/10240
                        time:   [16.746 µs 17.002 µs 17.260 µs]
                        thrpt:  [4.7461 GB/s 4.8183 GB/s 4.8918 GB/s]
```

### Profiler

```bash
# Install cargo flamegraph.
$ cargo install flamegraph

# Run benchmarks with profiler.
$ cargo flamegraph --bench ring -- --bench --profile-time 60
```