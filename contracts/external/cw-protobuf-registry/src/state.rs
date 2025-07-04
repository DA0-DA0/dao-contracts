use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};

/// Map protobuf file name -> protobuf file descriptor proto data.
pub const FILES: Map<String, Vec<u8>> = Map::new("files");

/// Map protobuf message descriptor name -> protobuf file name that contains it.
/// Secondary index on file_name to look up/iterate by file. This supports the
/// following queries:
/// - get a specific protobuf message by name (map lookup)
/// - list all protobuf messages (range query)
/// - list all protobuf messages in a specific file (secondary index range
///   query)
pub const MESSAGES: IndexedMap<String, String, MessagesIndexes<'_>> = IndexedMap::new(
    "messages",
    MessagesIndexes {
        file_name: MultiIndex::new(
            |_pk, file_name| file_name.clone(),
            "messages",
            "messages__file_name",
        ),
    },
);

/// Secondary index for protobuf descriptors to look up/iterate by file name.
pub struct MessagesIndexes<'a> {
    pub file_name: MultiIndex<'a, String, String, String>,
}
impl IndexList<String> for MessagesIndexes<'_> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<String>> + '_> {
        let v: Vec<&dyn Index<String>> = vec![&self.file_name];
        Box::new(v.into_iter())
    }
}

/// Map message name -> file descriptor set that contains the exact files with
/// the exact messages/enums needed to decode the message.
pub const PREPARED: Map<String, Vec<u8>> = Map::new("prepared");
