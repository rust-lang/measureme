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

### mmedit

`mmedit` is for editing `.mm_profdata` files generated by `measureme`.

[Learn more](./mmedit/README.md)

### mmview

`mmview` is for printing the event data generated by `measureme`.

[Learn more](./mmview/README.md)

### analyzeme

`analyzeme` is a library with common functionality for measureme tools.

[Learn more](./analyzeme/README.md)

### decodeme

`decodeme` holds the decoding definitions of the profiling event data from `measureme`.

[Learn more](./decodeme/README.md)

## How to make a release

1) Bump version of `measureme`, `decodeme` and `analyzeme` crates in the root `Cargo.toml` file
   - Update both `workspace.version` and `workspace.dependencies.[analyzeme/decodeme/measureme].version`
2) Update changelog with latest changes
   - You can use `https://github.com/rust-lang/measureme/compare/<last-released-tag>...master` to see what has changed since the last released tag
3) Merge a PR with the changes above (e.g. https://github.com/rust-lang/measureme/pull/240)
4) Create a git tag based on the merged PR, and push it
5) Create a GitHub release manually based on that tag, CI will then take care of publishing the crates
