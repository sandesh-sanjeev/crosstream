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
                        time:   [1.0533 µs 1.1308 µs 1.2067 µs]
                        thrpt:  [33.944 GB/s 36.223 GB/s 38.889 GB/s]

oracle/copy_from_slice/5120
                        time:   [1.0947 µs 1.1695 µs 1.2491 µs]
                        thrpt:  [32.791 GB/s 35.022 GB/s 37.417 GB/s]
```

### Profiler

```bash
# Install cargo flamegraph.
$ cargo install flamegraph

# Run benchmarks with profiler.
$ cargo flamegraph --bench hadron -- --bench --profile-time 60
```