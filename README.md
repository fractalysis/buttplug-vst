## Running

This VST is assuming there is some kind of Intiface server already running, so install https://intiface.com/desktop/ first, and make sure the server is running.

## Building

This VST uses baseplug, so to build it yourself will require being on the nightly toolchain before building:

```
rustup default nightly
cargo build
```