# Crosstream

`Crosstream` provides different types of ring buffers, along with primitives to build them yourself.

[![Build](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml)

[![Coverage Status](https://coveralls.io/repos/github/sandesh-sanjeev/crosstream/badge.svg?branch=master)](https://coveralls.io/github/sandesh-sanjeev/crosstream?branch=master)

Note that these test coverage numbers are not quite accurate. I haven't figured out
how to filter out benchmark functions from coverage yet either.

## Security

Crates makes liberal use of `unsafe` for better perf. However we use `unsafe`
only where it is trivially provable correct to human readers and proof engines. 

## Tests

```bash
# No features
$ cargo test

# Record with zerocopy based transmutation.
$ cargo test --features zerocopy

# Record with bytemuck based transmutation.
$ cargo test --features bytemuck

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
$ MIRIFLAGS=-Zmiri-disable-isolation cargo miri test --features zerocopy
$ MIRIFLAGS=-Zmiri-disable-isolation cargo miri test --features bytemuck

# Remove workspace override.
$ rustup override remove
```

## Benchmarks

On my Apple M1 Pro with 32 GB Memory.

* Size of record is 8 bytes (u64).
* About 8GB of total space for ring buffer (16_777_216 * 64).
* Append requires expensive input setup, making observations not very accurate.

```bash
$ cargo bench --features benchmark
Vec/append              time:   [391.84 ns 394.57 ns 397.54 ns]
                        thrpt:  [41.214 GB/s 41.524 GB/s 41.813 GB/s]

Vec/query               time:   [527.20 ns 530.77 ns 534.47 ns]
                        thrpt:  [30.655 GB/s 30.869 GB/s 31.077 GB/s]

OnHeap/append           time:   [401.79 ns 406.18 ns 411.25 ns]
                        thrpt:  [39.840 GB/s 40.337 GB/s 40.777 GB/s]

OnHeap/query            time:   [526.51 ns 529.39 ns 532.52 ns]
                        thrpt:  [30.767 GB/s 30.949 GB/s 31.118 GB/s]

OffHeap/append          time:   [425.32 ns 433.74 ns 445.10 ns]
                        thrpt:  [36.810 GB/s 37.773 GB/s 38.522 GB/s]

OffHeap/query           time:   [506.60 ns 508.22 ns 510.23 ns]
                        thrpt:  [32.111 GB/s 32.238 GB/s 32.341 GB/s]
```