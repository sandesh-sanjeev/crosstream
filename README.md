# Crosstream

`Crosstream` provides different types of ring buffers, along with primitives to build them yourself.

[![Build](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml)

[![Coverage Status](https://coveralls.io/repos/github/sandesh-sanjeev/crosstream/badge.svg?branch=master)](https://coveralls.io/github/sandesh-sanjeev/crosstream?branch=master)


## Security

Crates makes minimal use of `unsafe` for better perf. However we use `unsafe`
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