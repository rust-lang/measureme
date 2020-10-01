# Changelog

## [0.8.0] - 2020-10-01
### Added
- `analyzeme`: Profiling data can now be constructed directly from memory without having to touch the filesystem ([GH-123])
- `summarize`: A new "Time" column shows the total amount of time spent executing the query including sub-queries ([GH-109])

### Changed
- `crox`: Event argument data is now included in the output file ([GH-108])
- `measureme`: Trace data is now recorded into a single file instead of three files ([GH-132])
- `mmview`: Do not panic when there are no events ([GH-119])
- `summarize`: Time spent in incremental result cache loading and query blocking now counts toward self-time for the query ([GH-104])
- `summarize`: Improve support for loading trace files created by programs other than rustc ([GH-116])
- `summarize`: Only show the "Cache hits", "Blocked Time" and "Incremental load time" columns if that data is present in the trace ([GH-116])

## [0.7.1] - 2020-01-02
### Changed
- `measureme`: Fix compilation error on big endian systems ([GH-103])

## [0.7.0] - 2019-12-18
### Changed
- `measureme`: Events can now have "arguments" which record additional data about the event ([GH-101])

## [0.6.0] - 2019-12-11
### Added
- `measureme`: Added `SerializationSink::write_bytes_atomic` that optimizes handling of existing buffers ([GH-97])

### Changed
- `summarize`: Fixed a crash when incr_cache_load events would have child events ([GH-93])
- `measureme`: Replaced notion of "reserved" StringIds with simpler "virtual" StringIds ([GH-98])

## [0.5.0] - 2019-12-02
### Added
- `flamegraph`: new tool that uses the `inferno` crate to generate flamegraph svg files ([GH-73])
- `crox`: Added the `--dir` parameter to merge all events files in dir in to one trace file ([GH-84])
- `crox`: Added possibility to add multiple `file_prefix` parameters to merge all them to one trace file ([GH-84])
- `summarize`: Added self_time_change as percentage change of self_time from base to the `diff` sub command ([GH-87])

### Changed
- `measureme`: Stringtable data is recorded in a more compact format ([GH-90])
- `measureme`: Events are recorded in a more compact format ([GH-76])
- `stack_collapse`: Removed the `--interval` commandline option ([GH-76])

## [0.4.0] - 2019-10-24
### Added
- `measureme`: Added RAII-based API for recording events ([GH-70])
- `measureme`: Added support for compiling the library under wasm/wasi ([GH-43])
- `mmview`: Added the `-t` flag to limit output to results on the specified thread id ([GH-49])
- `summarize`: Added the `diff` sub command to compare two profiles ([GH-50])
- `crox`: Added the `--collapse-threads` flag to collapse events from unrelated threads to make visual analysis easier ([GH-56])
- `crox`: Added the `--minimum-duration` flag to filter out events under the specified number of microseconds ([GH-60])

### Changed
- `summarize`: Moved summarization under the `summarize` sub command ([GH-50])
- `crox`: Output files are now up to 50% smaller ([GH-59])

## [0.3.0] - 2019-05-14
### Added
- `summarize`: New CLI argument `percent-above` for `summarize` crate ([GH-32])
- `summarize`: Added documentation ([GH-35])
- `measureme`: Added a version tag to the binary event file format ([GH-41])

## [0.2.1] - 2019-04-12

## [0.2.0] - 2019-04-10

[0.8.0]: https://github.com/rust-lang/measureme/releases/tag/0.8.0
[0.7.1]: https://github.com/rust-lang/measureme/releases/tag/0.7.1
[0.7.0]: https://github.com/rust-lang/measureme/releases/tag/0.7.0
[0.6.0]: https://github.com/rust-lang/measureme/releases/tag/0.6.0
[0.5.0]: https://github.com/rust-lang/measureme/releases/tag/0.5.0
[0.4.0]: https://github.com/rust-lang/measureme/releases/tag/0.4.0
[0.3.0]: https://github.com/rust-lang/measureme/releases/tag/0.3.0
[0.2.1]: https://github.com/rust-lang/measureme/releases/tag/0.2.1
[0.2.0]: https://github.com/rust-lang/measureme/releases/tag/0.2.0

[GH-32]: https://github.com/rust-lang/measureme/issues/32
[GH-35]: https://github.com/rust-lang/measureme/pull/35
[GH-41]: https://github.com/rust-lang/measureme/pull/41
[GH-43]: https://github.com/rust-lang/measureme/pull/43
[GH-49]: https://github.com/rust-lang/measureme/pull/49
[GH-56]: https://github.com/rust-lang/measureme/pull/56
[GH-59]: https://github.com/rust-lang/measureme/pull/59
[GH-60]: https://github.com/rust-lang/measureme/pull/60
[GH-70]: https://github.com/rust-lang/measureme/pull/70
[GH-73]: https://github.com/rust-lang/measureme/pull/73
[GH-76]: https://github.com/rust-lang/measureme/pull/76
[GH-84]: https://github.com/rust-lang/measureme/pull/84
[GH-87]: https://github.com/rust-lang/measureme/pull/87
[GH-90]: https://github.com/rust-lang/measureme/pull/90
[GH-93]: https://github.com/rust-lang/measureme/pull/93
[GH-97]: https://github.com/rust-lang/measureme/pull/97
[GH-98]: https://github.com/rust-lang/measureme/pull/98
[GH-101]: https://github.com/rust-lang/measureme/pull/101
[GH-103]: https://github.com/rust-lang/measureme/pull/103
[GH-104]: https://github.com/rust-lang/measureme/pull/104
[GH-108]: https://github.com/rust-lang/measureme/pull/108
[GH-109]: https://github.com/rust-lang/measureme/pull/109
[GH-116]: https://github.com/rust-lang/measureme/pull/116
[GH-119]: https://github.com/rust-lang/measureme/pull/119
[GH-123]: https://github.com/rust-lang/measureme/pull/123
[GH-132]: https://github.com/rust-lang/measureme/pull/132
