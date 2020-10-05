//! See module-level documentation `measureme::stringtable`.

use measureme::stringtable::{METADATA_STRING_ID, TERMINATOR};
use measureme::{
    file_header::{
        strip_file_header, verify_file_header, FILE_MAGIC_STRINGTABLE_DATA,
        FILE_MAGIC_STRINGTABLE_INDEX,
    },
    stringtable::STRING_REF_ENCODED_SIZE,
    stringtable::STRING_REF_TAG,
};
use measureme::{Addr, StringId};
use memchr::{memchr, memchr2};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::convert::TryInto;
use std::error::Error;
use std::path::Path;

fn deserialize_index_entry(bytes: &[u8]) -> (StringId, Addr) {
    (
        StringId::new(u32::from_le_bytes(bytes[0..4].try_into().unwrap())),
        Addr(u32::from_le_bytes(bytes[4..8].try_into().unwrap())),
    )
}

#[derive(Copy, Clone)]
pub struct StringRef<'st> {
    id: StringId,
    table: &'st StringTable,
}

// This is the text we emit when encountering a virtual string ID that cannot
// be resolved.
const UNKNOWN_STRING: &str = "<unknown>";

// This is the text we emit when we encounter string data that does not have a
// proper terminator.
const INVALID_STRING: &str = "<invalid>";

impl<'st> StringRef<'st> {
    /// Expands the StringRef into an actual string. This method will
    /// avoid allocating a `String` if it can instead return a `&str` pointing
    /// into the raw string table data.
    pub fn to_string(&self) -> Cow<'st, str> {
        let addr = match self.get_addr() {
            Ok(addr) => addr,
            Err(_) => return Cow::from(UNKNOWN_STRING),
        };

        // Try to avoid the allocation, which we can do if this is
        //
        //  - a string with a single value component (`[value, 0xFF]`) or
        //  - a string with a single reference component (`[string_id, 0xFF]`)

        let pos = addr.as_usize();
        let slice_to_search = &self.table.string_data[pos..];

        // Find the first 0xFF byte which which is either the sequence
        // terminator or a byte in the middle of string id. Use `memchr` which
        // is super fast.
        let terminator_pos = memchr(TERMINATOR, slice_to_search).unwrap();

        // Check if this is a string containing a single StringId component
        let first_byte = self.table.string_data[pos];
        if first_byte == STRING_REF_TAG && terminator_pos == pos + STRING_REF_ENCODED_SIZE {
            let id = decode_string_ref_from_data(&self.table.string_data[pos..]);
            return StringRef {
                id,
                table: self.table,
            }
            .to_string();
        }

        // Decode the bytes until the terminator. If there is a string id in
        // between somewhere this will fail, and we fall back to the allocating
        // path.
        if let Ok(s) = std::str::from_utf8(&slice_to_search[..terminator_pos]) {
            Cow::from(s)
        } else {
            // This is the slow path where we actually allocate a `String` on
            // the heap and expand into that. If you suspect that there is a
            // bug in the fast path above, you can easily check if always taking
            // the slow path fixes the issue.
            let mut output = String::new();
            self.write_to_string(&mut output);
            Cow::from(output)
        }
    }

    pub fn write_to_string(&self, output: &mut String) {
        let addr = match self.get_addr() {
            Ok(addr) => addr,
            Err(_) => {
                output.push_str(UNKNOWN_STRING);
                return;
            }
        };

        let mut pos = addr.as_usize();

        loop {
            let byte = self.table.string_data[pos];

            if byte == TERMINATOR {
                return;
            } else if byte == STRING_REF_TAG {
                let string_ref = StringRef {
                    id: decode_string_ref_from_data(&self.table.string_data[pos..]),
                    table: self.table,
                };

                string_ref.write_to_string(output);

                pos += STRING_REF_ENCODED_SIZE;
            } else {
                // This is a literal UTF-8 string value. Find its end by looking
                // for either of the two possible terminator bytes.
                let remaining_data = &self.table.string_data[pos..];
                if let Some(len) = memchr2(0xFF, 0xFE, remaining_data) {
                    let value = String::from_utf8_lossy(&remaining_data[..len]);
                    output.push_str(&value);
                    pos += len;
                } else {
                    // The grammar does not allow unterminated raw strings. We
                    // have to stop decoding.
                    output.push_str(INVALID_STRING);
                    return;
                }
            }
        }
    }

    fn get_addr(&self) -> Result<Addr, ()> {
        if self.id.is_virtual() {
            match self.table.index.get(&self.id) {
                Some(&addr) => Ok(addr),
                None => Err(()),
            }
        } else if self.id == StringId::INVALID {
            Err(())
        } else {
            Ok(self.id.to_addr())
        }
    }
}

// String IDs in the table data are encoded in big endian format, while string
// IDs in the index are encoded in little endian format. Don't mix the two up.
fn decode_string_ref_from_data(bytes: &[u8]) -> StringId {
    // The code below assumes we use a 5-byte encoding for string
    // refs, where the first byte is STRING_REF_TAG and the
    // following 4 bytes are a little-endian u32 string ID value.
    assert!(bytes[0] == STRING_REF_TAG);
    assert!(STRING_REF_ENCODED_SIZE == 5);

    let id = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
    StringId::new(id)
}

/// Read-only version of the string table
#[derive(Debug)]
pub struct StringTable {
    // TODO: Replace with something lazy
    string_data: Vec<u8>,
    index: FxHashMap<StringId, Addr>,
}

impl StringTable {
    pub fn new(
        string_data: Vec<u8>,
        index_data: Vec<u8>,
        diagnostic_file_path: Option<&Path>,
    ) -> Result<StringTable, Box<dyn Error + Send + Sync>> {
        verify_file_header(
            &string_data,
            FILE_MAGIC_STRINGTABLE_DATA,
            diagnostic_file_path,
            "StringTable Data",
        )?;
        verify_file_header(
            &index_data,
            FILE_MAGIC_STRINGTABLE_INDEX,
            diagnostic_file_path,
            "StringTable Index",
        )?;

        assert!(index_data.len() % 8 == 0);
        let index: FxHashMap<_, _> = strip_file_header(&index_data)
            .chunks(8)
            .map(deserialize_index_entry)
            .collect();

        Ok(StringTable { string_data, index })
    }

    #[inline]
    pub fn get<'a>(&'a self, id: StringId) -> StringRef<'a> {
        StringRef { id, table: self }
    }

    pub fn get_metadata<'a>(&'a self) -> StringRef<'a> {
        let id = StringId::new(METADATA_STRING_ID);
        self.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use measureme::{PageTag, SerializationSinkBuilder, StringComponent, StringTableBuilder};
    use std::sync::Arc;

    #[test]
    fn simple_strings() {
        let sink_builder = SerializationSinkBuilder::new_in_memory();
        let data_sink = Arc::new(sink_builder.new_sink(PageTag::StringData));
        let index_sink = Arc::new(sink_builder.new_sink(PageTag::StringIndex));

        let expected_strings = &[
            "abc",
            "",
            "xyz",
            "g2h9284hgjv282y32983849&(*^&YIJ#R)(F83 f 23 2g4 35g5y",
            "",
            "",
            "g2h9284hgjv282y32983849&35g5y",
        ];

        let mut string_ids = vec![];

        {
            let builder = StringTableBuilder::new(data_sink.clone(), index_sink.clone()).unwrap();

            for &s in expected_strings {
                string_ids.push(builder.alloc(s));
            }
        }

        let data_bytes = Arc::try_unwrap(data_sink).unwrap().into_bytes();
        let index_bytes = Arc::try_unwrap(index_sink).unwrap().into_bytes();

        let string_table = StringTable::new(data_bytes, index_bytes, None).unwrap();

        for (&id, &expected_string) in string_ids.iter().zip(expected_strings.iter()) {
            let str_ref = string_table.get(id);

            assert_eq!(str_ref.to_string(), expected_string);

            let mut write_to = String::new();
            str_ref.write_to_string(&mut write_to);
            assert_eq!(str_ref.to_string(), write_to);
        }
    }

    #[test]
    fn composite_string() {
        let sink_builder = SerializationSinkBuilder::new_in_memory();
        let data_sink = Arc::new(sink_builder.new_sink(PageTag::StringData));
        let index_sink = Arc::new(sink_builder.new_sink(PageTag::StringIndex));

        let expected_strings = &[
            "abc",                  // 0
            "abcabc",               // 1
            "abcabcabc",            // 2
            "abcabcabc",            // 3
            "abcabcabc",            // 4
            "abcabcabcabc",         // 5
            "xxabcabcuuuabcabcqqq", // 6
            "xxxxxx",               // 7
        ];

        let mut string_ids = vec![];

        {
            let builder = StringTableBuilder::new(data_sink.clone(), index_sink.clone()).unwrap();

            let r = |id| StringComponent::Ref(id);
            let v = |s| StringComponent::Value(s);

            string_ids.push(builder.alloc("abc")); // 0
            string_ids.push(builder.alloc(&[r(string_ids[0]), r(string_ids[0])])); // 1
            string_ids.push(builder.alloc(&[r(string_ids[0]), r(string_ids[0]), r(string_ids[0])])); // 2
            string_ids.push(builder.alloc(&[r(string_ids[1]), r(string_ids[0])])); // 3
            string_ids.push(builder.alloc(&[r(string_ids[0]), r(string_ids[1])])); // 4
            string_ids.push(builder.alloc(&[r(string_ids[1]), r(string_ids[1])])); // 5
            string_ids.push(builder.alloc(&[
                v("xx"),
                r(string_ids[1]),
                v("uuu"),
                r(string_ids[1]),
                v("qqq"),
            ])); // 6
        }

        let data_bytes = Arc::try_unwrap(data_sink).unwrap().into_bytes();
        let index_bytes = Arc::try_unwrap(index_sink).unwrap().into_bytes();

        let string_table = StringTable::new(data_bytes, index_bytes, None).unwrap();

        for (&id, &expected_string) in string_ids.iter().zip(expected_strings.iter()) {
            let str_ref = string_table.get(id);

            assert_eq!(str_ref.to_string(), expected_string);

            let mut write_to = String::new();
            str_ref.write_to_string(&mut write_to);
            assert_eq!(str_ref.to_string(), write_to);
        }
    }
}
