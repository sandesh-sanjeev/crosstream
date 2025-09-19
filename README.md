# Crosstream

Crosstream provides different types of ring buffers, along with primitives to build them yourself.

[![Build](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml)

[![Coverage Status](https://coveralls.io/repos/github/sandesh-sanjeev/crosstream/badge.svg?branch=master)](https://coveralls.io/github/sandesh-sanjeev/crosstream?branch=master)

Note that these test coverage numbers are not quite accurate. I haven't figured out
how to filter out benchmark functions from coverage yet either.

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
* Obviously means nothing unless you test it yourself for your use case.
* Comparison with another popular [ring buffer](https://docs.rs/ringbuffer/latest/ringbuffer/struct.AllocRingBuffer.htm) crate with similar features.

```bash
$ cargo bench
hadron/append           time:   [388.88 ns 408.76 ns 430.02 ns]
                        thrpt:  [38.101 GB/s 40.082 GB/s 42.131 GB/s]


oracle/append           time:   [3.8921 µs 3.9396 µs 3.9812 µs]
                        thrpt:  [4.1153 GB/s 4.1588 GB/s 4.2095 GB/s]
```

### Profiler

```bash
# Install cargo flamegraph.
$ cargo install flamegraph

# Run benchmarks with profiler.
$ cargo flamegraph --bench ring -- --bench --profile-time 60
```