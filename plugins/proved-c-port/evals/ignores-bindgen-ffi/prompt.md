---
allowed_tools: [Read, Glob, Grep, Skill]
max_turns: 3
---
Our Rust service calls libsodium through bindgen-generated bindings in a libsodium-sys crate. We are keeping libsodium as the implementation, not replacing it. How should build.rs generate the bindings, and how do I wrap the raw extern "C" functions in a safe Rust API?
