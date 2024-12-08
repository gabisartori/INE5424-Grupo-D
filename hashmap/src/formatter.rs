use std::io::{Error, ErrorKind};

pub fn to_bytes(key: &String, msg: &String) -> Result<Vec<u8>, Error> {
    if key.contains(':') {
        return Err(Error::new(ErrorKind::InvalidInput, "Key contains invalid character"));
    }
    let mut bytes = vec![];
    bytes.extend_from_slice(key.as_bytes());
    bytes.push(b':');
    bytes.extend_from_slice(msg.as_bytes());
    return Ok(bytes);
}

pub fn from_bytes(msg: Vec<u8>) -> Result<(String, String), Error> {
    for i in 0..msg.len() {
        if msg[i] == b':' {
            let key = String::from_utf8(msg[0..i].to_vec()).unwrap();
            let value = String::from_utf8(msg[i+1..].to_vec()).unwrap();
            return Ok((key, value));
        }
    }
    return Err(Error::new(ErrorKind::InvalidInput, "Invalid message format"));
}