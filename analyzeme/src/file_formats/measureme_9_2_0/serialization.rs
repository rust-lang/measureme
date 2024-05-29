/// This module implements the "container" file format that `measureme` uses for
/// storing things on disk. The format supports storing three independent
/// streams of data: one for events, one for string data, and one for string
/// index data (in theory it could support an arbitrary number of separate
/// streams but three is all we need). The data of each stream is split into
/// "pages", where each page has a small header designating what kind of
/// data it is (i.e. event, string data, or string index), and the length of
/// the page.
///
/// Pages of different kinds can be arbitrarily interleaved. The headers allow
/// for reconstructing each of the streams later on. An example file might thus
/// look like this:
///
/// ```ignore
/// | file header | page (events) | page (string data) | page (events) | page (string index) |
/// ```
///
/// The exact encoding of a page is:
///
/// | byte slice              | contents                                |
/// |-------------------------|-----------------------------------------|
/// | &[0 .. 1]               | page tag                                |
/// | &[1 .. 5]               | page size as little endian u32          |
/// | &[5 .. (5 + page_size)] | page contents (exactly page_size bytes) |
///
/// A page is immediately followed by the next page, without any padding.
use rustc_hash::FxHashMap;
use std::convert::TryInto;
use std::fmt::Debug;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PageTag {
    Events = 0,
    StringData = 1,
    StringIndex = 2,
}

impl std::convert::TryFrom<u8> for PageTag {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PageTag::Events),
            1 => Ok(PageTag::StringData),
            2 => Ok(PageTag::StringIndex),
            _ => Err(format!("Could not convert byte `{}` to PageTag.", value)),
        }
    }
}

/// An address within a data stream. Each data stream has its own address space,
/// i.e. the first piece of data written to the events stream will have
/// `Addr(0)` and the first piece of data written to the string data stream
/// will *also* have `Addr(0)`.
//
// TODO: Evaluate if it makes sense to add a type tag to `Addr` in order to
//       prevent accidental use of `Addr` values with the wrong address space.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Addr(pub u32);

impl Addr {
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// This function reconstructs the individual data streams from their paged
/// version.
///
/// For example, if `E` denotes the page header of an events page, `S` denotes
/// the header of a string data page, and lower case letters denote page
/// contents then a paged stream could look like:
///
/// ```ignore
/// s = Eabcd_Sopq_Eef_Eghi_Srst
/// ```
///
/// and `split_streams` would result in the following set of streams:
///
/// ```ignore
/// split_streams(s) = {
///     events: [abcdefghi],
///     string_data: [opqrst],
/// }
/// ```
pub fn split_streams(paged_data: &[u8]) -> FxHashMap<PageTag, Vec<u8>> {
    let mut result: FxHashMap<PageTag, Vec<u8>> = FxHashMap::default();

    let mut pos = 0;
    while pos < paged_data.len() {
        let tag = TryInto::try_into(paged_data[pos]).unwrap();
        let page_size =
            u32::from_le_bytes(paged_data[pos + 1..pos + 5].try_into().unwrap()) as usize;

        assert!(page_size > 0);

        result
            .entry(tag)
            .or_default()
            .extend_from_slice(&paged_data[pos + 5..pos + 5 + page_size]);

        pos += page_size + 5;
    }

    result
}
