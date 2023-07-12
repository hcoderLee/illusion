use crate::block::{Hash, TimeStamp};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn get_timestamp() -> TimeStamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Transfer binary data to hex string
pub fn bytes2hex(bytes: &[u8]) -> String {
    let mut buf = String::from("0x");
    for x in bytes {
        buf.push_str(format!("{:02x}", x).as_str());
    }
    buf
}

/// Transfer hash to string
pub fn hash2str(hash: &Hash) -> String {
    let s = hash.map(|n| format!("{:02x}", n)).concat();
    format!("0x{}", s)
}
