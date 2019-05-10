//! A string table implementation with a tree-like encoding.
//!
//! Each entry in the table represents a string and encoded is a list of
//! components where each component can either be
//!
//! 1. a TAG_STR_VAL that contains actual string content,
//! 2. a TAG_STR_REF that contains a reference to another entry, or
//! 3. a TAG_TERMINATOR which marks the end of a component list.
//!
//! The string content of an entry is defined as the concatenation of the
//! content of its components. The content of a `TAG_STR_VAL` is its actual
//! UTF-8 bytes. The content of a `TAG_STR_REF` is the contents of the entry
//! it references.

use crate::file_header::{write_file_header, read_file_header, strip_file_header,
                         FILE_MAGIC_STRINGTABLE_DATA, FILE_MAGIC_STRINGTABLE_INDEX};
use crate::serialization::{Addr, SerializationSink};
use byteorder::{ByteOrder, LittleEndian};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::error::Error;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// A `StringId` is used to identify a string in the `StringTable`.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
#[repr(C)]
pub struct StringId(u32);

impl StringId {
    #[inline]
    pub fn reserved(id: u32) -> StringId {
        StringId(id)
    }
}

// Tags for the binary encoding of strings

/// Marks the end of a string component list.
const TAG_TERMINATOR: u8 = 0;

/// Marks a component that contains actual string data.
const TAG_STR_VAL: u8 = 1;

/// Marks a component that contains the ID of another string.
const TAG_STR_REF: u8 = 2;

const MAX_PRE_RESERVED_STRING_ID: u32 = std::u32::MAX / 2;

/// Write-only version of the string table
pub struct StringTableBuilder<S: SerializationSink> {
    data_sink: Arc<S>,
    index_sink: Arc<S>,
    id_counter: AtomicU32, // initialized to MAX_PRE_RESERVED_STRING_ID + 1
}

/// Anything that implements `SerializableString` can be written to a
/// `StringTable`.
pub trait SerializableString {
    fn serialized_size(&self) -> usize;
    fn serialize(&self, bytes: &mut [u8]);
}

// A simple string is encoded as
//
// [TAG_STR_VAL, len: u16, utf8_bytes, TAG_TERMINATOR]
//
// in the string table.
impl SerializableString for str {
    #[inline]
    fn serialized_size(&self) -> usize {
        1 + // tag
        2 + // len
        self.len() + // actual bytes
        1 // terminator
    }

    #[inline]
    fn serialize(&self, bytes: &mut [u8]) {
        assert!(self.len() <= std::u16::MAX as usize);
        let last_byte_index = bytes.len() - 1;
        bytes[0] = TAG_STR_VAL;
        LittleEndian::write_u16(&mut bytes[1..3], self.len() as u16);
        bytes[3..last_byte_index].copy_from_slice(self.as_bytes());
        bytes[last_byte_index] = TAG_TERMINATOR;
    }
}

/// A single component of a string. Used for building composite table entries.
pub enum StringComponent<'s> {
    Value(&'s str),
    Ref(StringId),
}

impl<'a> SerializableString for [StringComponent<'a>] {
    #[inline]
    fn serialized_size(&self) -> usize {
        unimplemented!()
    }

    #[inline]
    fn serialize(&self, _bytes: &mut [u8]) {
        unimplemented!()
    }
}

fn serialize_index_entry<S: SerializationSink>(sink: &S, id: StringId, addr: Addr) {
    sink.write_atomic(8, |bytes| {
        LittleEndian::write_u32(&mut bytes[0..4], id.0);
        LittleEndian::write_u32(&mut bytes[4..8], addr.0);
    });
}

fn deserialize_index_entry(bytes: &[u8]) -> (StringId, Addr) {
    (
        StringId(LittleEndian::read_u32(&bytes[0..4])),
        Addr(LittleEndian::read_u32(&bytes[4..8])),
    )
}

impl<S: SerializationSink> StringTableBuilder<S> {
    pub fn new(data_sink: Arc<S>, index_sink: Arc<S>) -> StringTableBuilder<S> {

        // The first thing in every file we generate must be the file header.
        write_file_header(&*data_sink, FILE_MAGIC_STRINGTABLE_DATA);
        write_file_header(&*index_sink, FILE_MAGIC_STRINGTABLE_INDEX);

        StringTableBuilder {
            data_sink,
            index_sink,
            id_counter: AtomicU32::new(MAX_PRE_RESERVED_STRING_ID + 1),
        }
    }

    #[inline]
    pub fn alloc_with_reserved_id<STR: SerializableString + ?Sized>(
        &self,
        id: StringId,
        s: &STR,
    ) -> StringId {
        assert!(id.0 <= MAX_PRE_RESERVED_STRING_ID);
        self.alloc_unchecked(id, s);
        id
    }

    #[inline]
    pub fn alloc<STR: SerializableString + ?Sized>(&self, s: &STR) -> StringId {
        let id = StringId(self.id_counter.fetch_add(1, Ordering::SeqCst));
        debug_assert!(id.0 > MAX_PRE_RESERVED_STRING_ID);
        self.alloc_unchecked(id, s);
        id
    }

    #[inline]
    fn alloc_unchecked<STR: SerializableString + ?Sized>(&self, id: StringId, s: &STR) {
        let size_in_bytes = s.serialized_size();
        let addr = self.data_sink.write_atomic(size_in_bytes, |mem| {
            s.serialize(mem);
        });

        serialize_index_entry(&*self.index_sink, id, addr);
    }
}

#[derive(Copy, Clone)]
pub struct StringRef<'st> {
    id: StringId,
    table: &'st StringTable,
}

impl<'st> StringRef<'st> {
    pub fn to_string(&self) -> Cow<'st, str> {
        let addr = self.table.index[&self.id].as_usize();
        let tag = self.table.string_data[addr];

        match tag {
            TAG_STR_VAL => {
                let len =
                    LittleEndian::read_u16(&self.table.string_data[addr + 1..addr + 3]) as usize;
                let next_component_addr = addr + 3 + len;
                let next_tag = self.table.string_data[next_component_addr];

                if next_tag == TAG_TERMINATOR {
                    let bytes = &self.table.string_data[addr + 3..addr + 3 + len];
                    return Cow::from(std::str::from_utf8(bytes).unwrap());
                }
            }
            TAG_TERMINATOR => {
                return Cow::from("");
            }
            _ => {
                // we have to take the allocating path
            }
        }

        let mut output = String::new();
        self.write_to_string(&mut output);
        Cow::from(output)
    }

    pub fn write_to_string(&self, output: &mut String) {
        let addr = self.table.index[&self.id];

        let mut pos = addr.as_usize();

        loop {
            let tag = self.table.string_data[pos];

            match tag {
                TAG_STR_VAL => {
                    pos += 1;
                    let len =
                        LittleEndian::read_u16(&self.table.string_data[pos..pos + 2]) as usize;
                    pos += 2;
                    let bytes = &self.table.string_data[pos..pos + len];
                    let s = std::str::from_utf8(bytes).unwrap();
                    output.push_str(s);
                    pos += len;
                }

                TAG_STR_REF => {
                    unimplemented!();
                }

                TAG_TERMINATOR => return,

                _ => unreachable!(),
            }
        }
    }
}

/// Read-only version of the string table
pub struct StringTable {
    // TODO: Replace with something lazy
    string_data: Vec<u8>,
    index: FxHashMap<StringId, Addr>,
}

impl<'data> StringTable {
    pub fn new(string_data: Vec<u8>, index_data: Vec<u8>) -> Result<StringTable, Box<dyn Error>> {

        let string_data_format = read_file_header(&string_data, FILE_MAGIC_STRINGTABLE_DATA)?;
        let index_data_format = read_file_header(&index_data, FILE_MAGIC_STRINGTABLE_INDEX)?;

        if string_data_format != index_data_format {
            Err("Mismatch between StringTable DATA and INDEX format version")?;
        }

        if string_data_format != 0 {
            Err(format!("StringTable file format version '{}' is not supported
                         by this version of `measureme`.", string_data_format))?;
        }

        assert!(index_data.len() % 8 == 0);
        let index: FxHashMap<_, _> = strip_file_header(&index_data)
            .chunks(8)
            .map(deserialize_index_entry)
            .collect();

        Ok(StringTable { string_data, index })
    }

    #[inline]
    pub fn get(&self, id: StringId) -> StringRef {
        StringRef { id, table: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_strings() {
        use crate::serialization::test::TestSink;

        let data_sink = Arc::new(TestSink::new());
        let index_sink = Arc::new(TestSink::new());

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
            let builder = StringTableBuilder::new(data_sink.clone(), index_sink.clone());

            for &s in expected_strings {
                string_ids.push(builder.alloc(s));
            }
        }

        let data_bytes = Arc::try_unwrap(data_sink).unwrap().into_bytes();
        let index_bytes = Arc::try_unwrap(index_sink).unwrap().into_bytes();

        let string_table = StringTable::new(data_bytes, index_bytes).unwrap();

        for (&id, &expected_string) in string_ids.iter().zip(expected_strings.iter()) {
            let str_ref = string_table.get(id);

            assert_eq!(str_ref.to_string(), expected_string);

            let mut write_to = String::new();
            str_ref.write_to_string(&mut write_to);
            assert_eq!(str_ref.to_string(), write_to);
        }
    }
}
