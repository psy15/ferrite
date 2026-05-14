#[derive(Debug, PartialEq)]
pub enum BencodeValue {
    Integer(i64),
    Bytes(Vec<u8>),
    List(Vec<BencodeValue>),
    Dict(Vec<(Vec<u8>, BencodeValue)>), // ordered, not HashMap
}

//Vec<u8> is just a growable list of bytes.
//u8 means unsigned 8-bit integer — exactly one byte, values 0–255.
//So Vec<u8> is how Rust represents raw binary data, which is exactly what bencode strings are.

pub fn decode(input: &[u8]) -> BencodeValue {
    // read bytes, return one of the 4 variants
    match input[0] {
        b'i' => {
            let end = input.iter().position(|&b| b == b'e').unwrap();
            let bytes = &input[1..end];
            let string = std::str::from_utf8(bytes).unwrap();
            let n = string.parse::<i64>().unwrap();
            BencodeValue::Integer(n)
        }
        _ => todo!(), // b'l'        => // parse list
                      // b'd'        => // parse dict
                      // b'0'..=b'9' => // parse string
                      // _           => // error, unexpected byte
    }
}

// important:
// BencodeValue::Integer(n) → returns it
// BencodeValue::Integer(n); → throws it away

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer() {
        let input = b"i42e";
        let result: BencodeValue = decode(input);
        assert_eq!(result, BencodeValue::Integer(42));
    }

    #[test]
    fn test_negative_integer() {
        let result = decode(b"i-3e");
        assert_eq!(result, BencodeValue::Integer(-3));
    }

    #[test]
    fn test_zero() {
        let result = decode(b"i0e");
        assert_eq!(result, BencodeValue::Integer(0));
    }
}
