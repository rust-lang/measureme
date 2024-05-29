//! A string table implementation with a tree-like encoding.
//!
//! Each entry in the table represents a string and is encoded as a list of
//! components where each component can either be
//!
//! 1. a string _value_ that contains actual UTF-8 string content,
//! 2. a string _ID_ that contains a reference to another entry, or
//! 3. a terminator tag which marks the end of a component list.
//!
//! The string _content_ of an entry is defined as the concatenation of the
//! content of its components. The content of a string value is its actual
//! UTF-8 bytes. The content of a string ID is the contents of the entry
//! it references.
//!
//! The byte-level encoding of component lists uses the structure of UTF-8 in
//! order to save space:
//!
//! - A valid UTF-8 codepoint never starts with the byte `0xFE`. We make use
//!   of this fact by letting all string ID components start with this `0xFE`
//!   prefix. Thus when we parse the contents of a value we know to stop if
//!   we encounter this byte.
//!
//! - A valid UTF-8 string cannot contain the `0xFF` byte. Thus we can safely
//!   use `0xFF` as our component list terminator.
//!
//! The sample composite string ["abc", ID(42), "def", TERMINATOR] would thus be
//! encoded as:
//!
//! ```ignore
//!     ['a', 'b' , 'c', 254, 42, 0, 0, 0, 'd', 'e', 'f', 255]
//!                      ^^^^^^^^^^^^^^^^                 ^^^
//!                 string ID with 0xFE prefix      terminator (0xFF)
//! ```
//!
//! As you can see string IDs are encoded in little endian format.
//!
//! ----------------------------------------------------------------------------
//!
//! Each string in the table is referred to via a `StringId`. `StringId`s may
//! be generated in two ways:
//!
//!   1. Calling `StringTableBuilder::alloc()` which returns the `StringId` for
//!      the allocated string.
//!   2. Calling `StringId::new_virtual()` to create a "virtual" `StringId` that
//!      later can be mapped to an actual string via
//!      `StringTableBuilder::map_virtual_to_concrete_string()`.
//!
//! String IDs allow you to deduplicate strings by allocating a string
//! once and then referring to it by id over and over. This is a useful trick
//! for strings which are recorded many times and it can significantly reduce
//! the size of profile trace files.
//!
//! `StringId`s are partitioned according to type:
//!
//! > [0 .. MAX_VIRTUAL_STRING_ID, METADATA_STRING_ID, .. ]
//!
//! From `0` to `MAX_VIRTUAL_STRING_ID` are the allowed values for virtual strings.
//! After `MAX_VIRTUAL_STRING_ID`, there is one string id (`METADATA_STRING_ID`)
//! which is used internally by `measureme` to record additional metadata about
//! the profiling session. After `METADATA_STRING_ID` are all other `StringId`
//! values.

use super::serialization::Addr;

/// A `StringId` is used to identify a string in the `StringTable`. It is
/// either a regular `StringId`, meaning that it contains the absolute address
/// of a string within the string table data. Or it is "virtual", which means
/// that the address it points to is resolved via the string table index data,
/// that maps virtual `StringId`s to addresses.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
#[repr(C)]
pub struct StringId(u32);

impl StringId {
    pub const INVALID: StringId = StringId(INVALID_STRING_ID);

    #[inline]
    pub fn new(id: u32) -> StringId {
        StringId(id)
    }

    #[inline]
    pub fn is_virtual(self) -> bool {
        self.0 <= METADATA_STRING_ID
    }

    #[inline]
    pub fn to_addr(self) -> Addr {
        Addr(self.0.checked_sub(FIRST_REGULAR_STRING_ID).unwrap())
    }
}

// See module-level documentation for more information on the encoding.
pub const TERMINATOR: u8 = 0xFF;
pub const STRING_REF_TAG: u8 = 0xFE;
pub const STRING_REF_ENCODED_SIZE: usize = 5;

/// The maximum id value a virtual string may be.
const MAX_USER_VIRTUAL_STRING_ID: u32 = 100_000_000;

/// The id of the profile metadata string entry.
pub const METADATA_STRING_ID: u32 = MAX_USER_VIRTUAL_STRING_ID + 1;

/// Some random string ID that we make sure cannot be generated or assigned to.
const INVALID_STRING_ID: u32 = METADATA_STRING_ID + 1;

pub const FIRST_REGULAR_STRING_ID: u32 = INVALID_STRING_ID + 1;
