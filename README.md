# measureme [![Rust](https://github.com/rust-lang/measureme/actions/workflows/rust.yml/badge.svg)](https://github.com/rust-lang/measureme/actions/workflows/rust.yml)
Support crate for rustc's self-profiling feature

This crate is maintained by the Rust compiler team and in particular by the
[self-profile working group][wg-self-profile]. It is currently only meant to
be used within rustc itself, so APIs may change at any moment.

## Tools

### measureme

`measureme` is the core library which contains a fast, efficient framework for recording events and serializing them to a compact binary format. It is integrated into `rustc` via the unstable `-Z self-profile` flag.

[Documentation](https://docs.rs/measureme)

### summarize

`summarize` produces a human readable summary of `measureme` profiling data.
It contains two main modes:

- `summarize` which groups the profiling events and orders the results by time taken.
- `diff` which compares two profiles and outputs a summary of the differences.

[Learn more](./summarize/README.md)

### stack_collapse

`stack_collapse` reads `measureme` profiling data and outputs folded stack traces compatible with the [Flame Graph](https://github.com/brendangregg/FlameGraph) tools.

[Learn more](./stack_collapse/README.md)

### flamegraph

`flamegraph` reads `measureme` profiling data and outputs [Flame Graph](https://github.com/brendangregg/FlameGraph).

[Learn more](./flamegraph/README.md)

### crox

`crox` turns `measureme` profiling data into files that can be visualized by the Chromium performance tools.

[Learn more](./crox/README.md)

[wg-self-profile]: https://rust-lang.github.io/compiler-team/working-groups/self-profile/
