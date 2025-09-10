# Crosstream

[![Build](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml)

[![Coverage Status](https://coveralls.io/repos/github/sandesh-sanjeev/crosstream/badge.svg?branch=master)](https://coveralls.io/github/sandesh-sanjeev/crosstream?branch=master)

## crosstream-ring

`crosstream-ring` provides ring buffers, along with primitives to build them yourself.

## Security

Crates makes minimal use of `unsafe` for better perf. However we use `unsafe`
only where it is trivially provable correct to human readers and proof engines. 

## Kani Verifier

<https://model-checking.github.io/kani/>

### Install

```bash
$ cargo install --locked kani-verifier
```

### Configure

```bash
$ cargo kani setup
[0/5] Running Kani first-time setup...
[1/5] Ensuring the existence of: /Users/sandeshsanjeev/.kani/kani-0.65.0
[2/5] Downloading Kani release bundle: kani-0.65.0-aarch64-apple-darwin.tar.gz
[3/5] Installing rust toolchain version: nightly-2025-08-06-aarch64-apple-darwin
info: syncing channel updates for 'nightly-2025-08-06-aarch64-apple-darwin'
...
```

### Run tests

Note that it takes a while to execute all the tests (at least one my M1 mac pro).

```bash
$ cargo kani --tests
...
Manual Harness Summary:
Complete - 52 successfully verified harnesses, 0 failures, 52 total.
```

#### With coverage

```bash
cargo kani --tests --coverage -Z source-coverage
...
...
crosstream-ring/src/segment.rs (segment::Segment::<segment::VecStorage<u64>>::with_capacity)
 * 204:5 - 211:6 COVERED

crosstream-ring/src/segment.rs (segment::verification::push_proof)
 * 316:5 - 325:34 COVERED
 * 326:13 - 330:10 COVERED
 * 326:23 - 326:31 COVERED
 * 333:9 - 345:6 COVERED

Verification Time: 10.441694s
```

#### Individual proof

```bash
$ cargo kani --tests --harness push_with_trim_proof
$ cargo kani --tests --harness push_with_trim_proof --coverage -Z source-coverage
```