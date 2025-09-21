# Crosstream

Crosstream provides high performance ring buffers.

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

```bash
$ cargo bench
hadron/copy_from_slice/5120
                        time:   [890.59 ns 937.80 ns 988.07 ns]
                        thrpt:  [41.455 GB/s 43.677 GB/s 45.992 GB/s]

oracle/copy_from_slice/5120
                        time:   [1.3498 µs 1.4415 µs 1.5264 µs]
                        thrpt:  [26.835 GB/s 28.414 GB/s 30.345 GB/s]
```

### Profiler

```bash
# Install cargo flamegraph.
$ cargo install flamegraph

# Run benchmarks with profiler.
$ cargo flamegraph --bench hadron -- --bench --profile-time 60
```