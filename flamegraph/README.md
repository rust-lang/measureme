# flamegraph

flamegraph is a tool to produce [Flame Graph](https://github.com/brendangregg/FlameGraph) from `measureme` data.

## Example

```bash
# Install flamegraph if you haven't done so yet.

$ cargo install --git https://github.com/rust-lang/measureme --branch stable flamegraph

$ git clone https://github.com/rust-lang/regex.git

$ cd regex

$ cargo rustc -- -Z self-profile

$ flamegraph regex-{pid}.mm_profdata

$ open rustc.svg
```
