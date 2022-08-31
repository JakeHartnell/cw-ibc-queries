use cw_storage_plus::Item;

pub const PENDING: Item<String> = Item::new("pending");
pub const PACKET_LIFETIME: Item<u64> = Item::new("packet_lifetime");
