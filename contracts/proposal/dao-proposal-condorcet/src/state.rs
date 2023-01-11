use cw_storage_plus::Map;

use crate::tally::Tally;

pub const TALLYS: Map<u32, Tally> = Map::new("t");
