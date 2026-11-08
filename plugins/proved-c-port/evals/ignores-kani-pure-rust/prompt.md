---
allowed_tools: [Read, Glob, Grep, Skill]
max_turns: 3
---
My pure Rust crate implements a fixed-capacity ring buffer; there is no C code and no FFI anywhere in it. How do I write Kani harnesses that prove push and pop never panic or index out of bounds, and how should I set the loop unwind bound?
