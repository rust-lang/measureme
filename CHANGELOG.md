# Changelog

## [9.2.0] - 2021-09-13
### Changed
- `analyzeme`: Makes a couple of methods in ProfilingData public. ([GH-180])

## [9.1.2] - 2021-05-21
### Added
- `measureme`: Allow recording interval events without using the drop guard ([GH-159])

## [9.1.1] - 2021-04-23
### Changed
- `crox`: Update the `--dir` flag to look for the correct file extension for traces ([GH-155])
- `measureme`: Update the `memmap` dependency to `memmap2` which is actively maintained ([GH-156])

## [9.1.0] - 2021-02-19
### Added
- `measureme`: Add support for using hardware performance counters instead of wall-clock times. ([GH-143])
- `summarize`: Add `aggregate` sub-command for analyzing sets of profiles ([GH-129])

### Changed
- `analyzeme`: Provide functions to decode paged buffer data from memory ([GH-142])
- `analyzeme`: Fix blocked events not being counted in total invocation count ([GH-148])
- `analyzeme`: Return error instead of panicking if the input file is too small ([GH-151])
- Cleanup intra-doc links ([GH-146])

## [9.0.0] - 2020-10-07
### Added
- `measureme`: Added a function to create `EventId`s with multiple arguments ([GH-138])

### Changed
- We now use the standard semantic versioning system. As this is the 9th breaking change, we're adopting `9.0` as the version number
- `measureme`: Allow recording up to 4gb of string data instead of the old limit of 1gb ([GH-137])

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

[9.2.0]: https://github.com/rust-lang/measureme/releases/tag/9.2.0
[9.1.2]: https://github.com/rust-lang/measureme/releases/tag/9.1.2
[9.1.1]: https://github.com/rust-lang/measureme/releases/tag/9.1.1
[9.1.0]: https://github.com/rust-lang/measureme/releases/tag/9.1.0
[9.0.0]: https://github.com/rust-lang/measureme/releases/tag/9.0.0
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
[GH-129]: https://github.com/rust-lang/measureme/pull/129
[GH-132]: https://github.com/rust-lang/measureme/pull/132
[GH-137]: https://github.com/rust-lang/measureme/pull/137
[GH-138]: https://github.com/rust-lang/measureme/pull/138
[GH-142]: https://github.com/rust-lang/measureme/pull/142
[GH-143]: https://github.com/rust-lang/measureme/pull/143
[GH-146]: https://github.com/rust-lang/measureme/pull/146
[GH-148]: https://github.com/rust-lang/measureme/pull/148
[GH-151]: https://github.com/rust-lang/measureme/pull/151
[GH-155]: https://github.com/rust-lang/measureme/pull/155
[GH-156]: https://github.com/rust-lang/measureme/pull/156
[GH-159]: https://github.com/rust-lang/measureme/pull/159
[GH-180]: https://github.com/rust-lang/measureme/pull/180
