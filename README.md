# Crosstream

`Crosstream` provides different types of ring buffers, along with primitives to build them yourself.

[![Build](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml)

[![Coverage Status](https://coveralls.io/repos/github/sandesh-sanjeev/crosstream/badge.svg?branch=master)](https://coveralls.io/github/sandesh-sanjeev/crosstream?branch=master)

Note that these test coverage numbers are not quite accurate.

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
$ cargo bench --features zerocopy
Gnuplot not found, using plotters backend
Vec/append              time:   [416.51 ns 436.75 ns 461.96 ns]
                        thrpt:  [35.466 GB/s 37.513 GB/s 39.336 GB/s]

Vec/query               time:   [504.24 ns 507.03 ns 510.31 ns]
                        thrpt:  [32.106 GB/s 32.313 GB/s 32.492 GB/s]

OnHeap/append           time:   [377.80 ns 383.03 ns 388.54 ns]
                        thrpt:  [42.168 GB/s 42.775 GB/s 43.367 GB/s]

OnHeap/query            time:   [503.32 ns 504.69 ns 506.40 ns]
                        thrpt:  [32.354 GB/s 32.464 GB/s 32.552 GB/s]

OffHeap/append          time:   [438.49 ns 476.71 ns 532.09 ns]
                        thrpt:  [30.792 GB/s 34.369 GB/s 37.364 GB/s]

OffHeap/query           time:   [514.11 ns 517.29 ns 520.72 ns]
                        thrpt:  [31.464 GB/s 31.673 GB/s 31.868 GB/s]
```