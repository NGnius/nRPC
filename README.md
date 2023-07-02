[![nRPC](https://img.shields.io/crates/v/nrpc?label=nrpc&style=flat-square)](https://crates.io/crates/nrpc)
[![nRPC-build](https://img.shields.io/crates/v/nrpc-build?label=nrpc-build&style=flat-square)](https://crates.io/crates/nrpc-build)

# nRPC

NG's custom spin of gRPC. Intended to be decoupled from the network layer for use with websockets.

# About

nRPC provides the glue logic from protobuf declarations to client and server Rust code. The server-side logic and client-server networking is not implemented. This makes it almost, but not quite, a gRPC implementation in Rust. To really drive that idea home, nRPC stands for nRPC Remote Procedure Call -- almost like what gRPC stands for.

Since the network layer is not provided, this will never be fully compliant with gRPC specifications. On the other hand, gRPC can't be used in browsers but nRPC could be used to write [something that does](https://github.com/NGnius/usdpl-rs). Since nRPC is just a hobby project, think of it like a cheap knock-off -- compliance with gRPC is best-effort where possible.

# Why?

I wanted a well-known RPC library that could work with a client in a browser. The most popular RPC library seemed to be gRPC, except that didn't support browsers. So I made something that fit my requirements.
