# decodeme

`decodeme` holds the decoding definitions of the profiling event data from `measureme`.

This makes it easy in the future to keep supporting old file formats by making
`analyzeme` depend on multiple versions of decodeme and always have it convert
data to the current format.

As an example, this is what the crate graph would look like for `analyzeme@17.0.0`
if we want it to support a couple of older file formats.

```text
measureme_15_0_0 <--- decodeme_15_0_0 <----+
                                           |
measureme_16_0_0 <--- decodeme_16_0_0 <----+
                                           |
measureme_17_0_0 <--- decodeme_17_0_0 <----+---- analyzeme_17_0_0
```

See [analyzeme/src/file_formats/v7.rs](../analyzeme/src/file_formats/v7.rs) for
an example of what it looks like to implement support for an old file format.
