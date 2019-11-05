# flamegraph

flamegraph is a tool to produce [Flame Graph](https://github.com/brendangregg/FlameGraph) from `measureme` data.

## Example

```bash
$ git clone https://github.com/rust-lang/regex.git

$ cd regex

$ cargo rustc -- -Z self-profile

$ flamegraph pid-{pid}

$ open rustc.svg
```
