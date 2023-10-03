Client for the Debian bug tracking system
=========================================

This crate hosts a simple rust wrapper around the [SOAP API for the Debian bug
tracking system](https://wiki.debian.org/DebbugsSoapInterface).

Example:

```rust

let client = debbugs::Debbugs::default();  // Connect to default instance
client
