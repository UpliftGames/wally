# Wally Registry Backend
This directory contains the backend to the Wally registry. It's the interface that clients use for downloading, publishing, and yanking packages.

## Requirements
- Rust 1.50.0+
- C toolchain for OpenSSL

## Running
A Dockerfile that builds and runs the backend in located in [backend.Dockerfile][Dockerfile] in the root of this repository.

For development, running this project is easy:

``` bash
cargo run
```

## Configuration
TODO

[Dockerfile]: ../backend.Dockerfile
