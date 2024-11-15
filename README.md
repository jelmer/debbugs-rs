Client for the Debian bug tracking system
=========================================

This create hosts a simple rust wrapper around the [SOAP API for the Debian bug
tracking system](https://wiki.debian.org/DebbugsSoapInterface).

Example:

```rust

let client = debbugs::Debbugs::default();  // Connect to default instance
println!("Lastest 10 bugs: {:?}", client.newest_bugs(10));
```

There are two separate interfaces, one in debbugs::Debbugs, which is
async - and one in debbugs::blocking::Debbugs.
