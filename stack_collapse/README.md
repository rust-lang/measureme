# stack-collapse

stack-collapse is a tool to produce [Flame Graph](https://github.com/brendangregg/FlameGraph) compatible folded stacks from `measureme` data.

## Example

```bash
$ git clone https://github.com/rust-lang/regex.git

$ cd regex

$ cargo rustc -- -Z self-profile

$ stack-collapse pid-{pid}

$ ../path/to/FlameGraph/flamegraph.pl out.stacks_folded > rustc.svg

$ open rustc.svg
```
