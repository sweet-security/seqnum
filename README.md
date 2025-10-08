# Serial Number Arithmetics

This simple (zero deps) crate provides `SequenceInt`, a wrapper around `u8`/`u16`/`u32`/`u64` 
that implements [serial number arithmetic](https://en.wikipedia.org/wiki/Serial_number_arithmetic) (as defined by [RFC 1982](https://datatracker.ietf.org/doc/html/rfc1982)) in rust.

This is useful for sequence numbers, like in TCP, where one would want to be able to order packets
even though their sequence numbers wrap around. Taking `u16` for instance, `0xfffe_u16 < 0xffff_u16 <  0_u16`.

```rust
use serial_int::SeqU16;

// thiss falls below the mid-point between the two numbers, making 1000 smaller
assert!(SeqU16::from(1000) < SeqU16::from(33000));

// this crosses the mid-point between the two numbers, making 1000 greater
assert!(SeqU16::from(1000) > SeqU16::from(34000));
```
