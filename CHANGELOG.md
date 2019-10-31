# Changelog

## Unreleased
### Added
- `flamegraph`: new tool that uses the `inferno` crate to generate flamegraph svg files ([GH-73])

### Changed
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
