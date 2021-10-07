# summarize

Summarize is a tool to produce a human readable summary of `measureme` profiling data.

## Installing summarize

To use this tool you will first want to install it:

```bash
$ cargo install --git https://github.com/rust-lang/measureme --branch stable summarize
```

## Profiling the nightly compiler

To profile the nightly compiler first ensure that you have a recent nightly compiler by
typing `rustup update nightly`. If your compiler version is older than `2019-04-13` the
profiling feature has not yet been added.

Profiling the compiler is done by passing the flag `-Z self-profile` to `rustc`. Note that
`-Z` flags are unstable, so you must use the nightly compiler. As an example we will
profile the [regex][regex-crate] crate.

[regex-crate]: https://github.com/rust-lang/regex

```bash
$ git clone https://github.com/rust-lang/regex.git
$ cd regex
$ cargo +nightly rustc -- -Z self-profile
```

The commands above will run `rustc` with the flag that enables profiling. You should now
have a file in your directory named `regex-{pid}.mm_profdata` which contains the profiler data. (If
you got three files instead, you will need to use an older version of the `summarize` tool such as
the `0.7.1` release to read the data:
`cargo install --git https://github.com/rust-lang/measureme --tag 0.7.1 summarize`)

You can now use the `summarize` tool we installed in the previous section to view the
contents of these files:

```bash
$ summarize summarize regex-{pid}.mm_profdata
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| Item                   | Self time | % of total time | Item count | Cache hits | Blocked time | Incremental load time |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| LLVM_emit_obj          | 4.51s     | 41.432          | 141        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| LLVM_module_passes     | 1.05s     | 9.626           | 140        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| LLVM_make_bitcode      | 712.94ms  | 6.543           | 140        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| typeck_tables_of       | 542.23ms  | 4.976           | 17470      | 16520      | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| codegen                | 366.82ms  | 3.366           | 141        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| optimized_mir          | 188.22ms  | 1.727           | 11668      | 9114       | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| mir_built              | 156.30ms  | 1.434           | 2040       | 1020       | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| evaluate_obligation    | 151.95ms  | 1.394           | 33134      | 23817      | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| LLVM_compress_bitcode  | 126.55ms  | 1.161           | 140        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| codegen crate          | 119.08ms  | 1.093           | 1          | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| mir_const              | 117.82ms  | 1.081           | 1050       | 30         | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+

(rows elided)

Total cpu time: 10.896488447s
```

## Profiling your own build of rustc

You can also profile your own custom build of rustc. First you'll have to clone the
[rust][rust-repo] repo and compile it. You can find the full guide on doing this
[here][compiling-rust], but if you've never built rustc before, we suggest starting with

[rust-repo]: https://github.com/rust-lang/rust
[compiling-rust]: https://rustc-dev-guide.rust-lang.org/building/how-to-build-and-run.html

```bash
$ git clone https://github.com/rust-lang/rust.git
$ ./x.py build
# This will take a while...
$ rustup toolchain link mytoolchain build/x86_64-unknown-linux-gnu/stage1
```

Where `mytoolchain` is the name of your custom toolchain. Now we do more or less the same
as before: (with regex as example)

```bash
$ git clone https://github.com/rust-lang/regex.git
$ cd regex
$ cargo +mytoolchain rustc -- -Z self-profile
$ summarize summarize regex-{pid}.mm_profdata
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| Item                   | Self time | % of total time | Item count | Cache hits | Blocked time | Incremental load time |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| LLVM_emit_obj          | 4.51s     | 41.432          | 141        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| LLVM_module_passes     | 1.05s     | 9.626           | 140        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| LLVM_make_bitcode      | 712.94ms  | 6.543           | 140        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| typeck_tables_of       | 542.23ms  | 4.976           | 17470      | 16520      | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| codegen                | 366.82ms  | 3.366           | 141        | 0          | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| optimized_mir          | 188.22ms  | 1.727           | 11668      | 9114       | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+
| mir_built              | 156.30ms  | 1.434           | 2040       | 1020       | 0.00ns       | 0.00ns                |
+------------------------+-----------+-----------------+------------+------------+--------------+-----------------------+

(rows elided)

Total cpu time: 10.896488447s
```

Note that your custom build of the compiler must not use a newer version of the
`measureme` library than the one used in the `summarize` tool.

## Reading the output

The table is a list of different events. Each event has its own row, and the columns
summarize the information for that event.

 * The `Item` column contains the name of the event.
 * The `Self time` column contains the total time used by events of this type.
 * The `% of total time` column contains how large a percentage `Self time` is of the
   total runtime of the compiler.
 * The `Item count` column describes the number of times that event has occurred.
 * The `Cache hits` column displays the number of times a [query][query] was found in the cache.
 * The `Blocked time` is the amount of time this event spent while waiting on a different
   thread. (This only happens with parallel queries enabled)
 * The `Incremental load time` is the time spent loading the result of a query from a
   previous incremental build. This is analogous to `Cache hits`.

[query]: https://rustc-dev-guide.rust-lang.org/query.html

The table is sorted by `Self time` descending.

## The `diff` sub command

The `diff` sub command allows you to compare the performance of two different profiles by event.

The output is a table like that of the `summarize` sub command but it instead shows the differences in each metric.

```bash
$ summarize diff base-profile.mm_profdata changed-profile.mm_profdata
+---------------------------+--------------+------------+------------+--------------+-----------------------+
| Item                      | Self Time    | Item count | Cache hits | Blocked time | Incremental load time |
+---------------------------+--------------+------------+------------+--------------+-----------------------+
| LLVM_module_passes        | -66.626471ms | +0         | +0         | +0ns         | +0ns                  |
+---------------------------+--------------+------------+------------+--------------+-----------------------+
| LLVM_emit_obj             | -38.700719ms | +0         | +0         | +0ns         | +0ns                  |
+---------------------------+--------------+------------+------------+--------------+-----------------------+
| LLVM_make_bitcode         | +32.006706ms | +0         | +0         | +0ns         | +0ns                  |
+---------------------------+--------------+------------+------------+--------------+-----------------------+
| mir_borrowck              | -12.808322ms | +0         | +0         | +0ns         | +0ns                  |
+---------------------------+--------------+------------+------------+--------------+-----------------------+
| typeck_tables_of          | -10.325247ms | +0         | +0         | +0ns         | +0ns                  |
+---------------------------+--------------+------------+------------+--------------+-----------------------+
(rows elided)
Total cpu time: -155.177548ms
```

The table is sorted by the absolute value of `Self time` descending.
