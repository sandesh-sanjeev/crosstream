# Crosstream

[![Build](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sandesh-sanjeev/crosstream/actions/workflows/rust.yml)

[![Coverage Status](https://coveralls.io/repos/github/sandesh-sanjeev/crosstream/badge.svg?branch=master)](https://coveralls.io/github/sandesh-sanjeev/crosstream?branch=master)

## crosstream-ring

`crosstream-ring` provides ring buffers, along with primitives to build them yourself.

## Security

Crates makes minimal use of `unsafe` for better perf. However we use `unsafe`
only where it is trivially provable correct to human readers and proof engines. 
