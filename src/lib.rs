mod serialization;
mod stringtable;

pub use crate::serialization::{Addr, SerializationSink};
pub use crate::stringtable::{
    SerializableString, StringId, StringRef, StringTable, StringTableBuilder,
};
