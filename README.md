[![Build Status](https://github.com/idanarye/bevy-yoetz/workflows/CI/badge.svg)](https://github.com/idanarye/bevy-yoetz/actions)
[![Latest Version](https://img.shields.io/crates/v/bevy-yoetz.svg)](https://crates.io/crates/bevy-yoetz)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://idanarye.github.io/bevy-yoetz/)

# Bevy Yoetz

Yoetz is a decision-making AI for the Bevy game engine.

## Features

* Describe the various strategies the AI agent can use by using a device macro on an `enum`.
* Write systems that suggest and score behaviors from that `enum`.
* AI informs user systems on the decision using the ECS (adds and removes components based on the strategy it chose)
* Suggestions can carry data that will appear in the strategy components. The exact behavior of that data can be customized in the derive macro.

## Example

Code: examples/example.rs

WASM: https://idanarye.github.io/bevy-yoetz/demos/example

Use the arrow keys to move the yellow square. The red square is controlled by AI. The AI's status is displayed above it.

https://github.com/idanarye/bevy-yoetz/assets/1149255/ad98e48f-8c86-451d-9a0f-82d9f6d1bac2

## Versions

| bevy | bevy-yoetz |
|------|------------|
| 0.13 | 0.1        |

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
