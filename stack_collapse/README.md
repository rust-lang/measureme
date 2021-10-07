# stack-collapse

stack-collapse is a tool to produce [Flame Graph](https://github.com/brendangregg/FlameGraph) compatible folded stacks from `measureme` data.

## Example

```bash
$ # Install stack_collapse if you haven't done so yet.

$ cargo install --git https://github.com/rust-lang/measureme --branch stable stack_collapse

$ git clone https://github.com/rust-lang/regex.git

$ cd regex

$ cargo rustc -- -Z self-profile

$ stack_collapse regex-{pid}.mm_profdata

$ ../path/to/FlameGraph/flamegraph.pl out.stacks_folded > rustc.svg

$ open rustc.svg
```
