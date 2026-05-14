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

        b'0'..=b'9' => {
            let colon = input.iter().position(|&b| b == b':').unwrap();
            let length: usize = std::str::from_utf8(&input[0..colon])
                .unwrap()
                .parse()
                .unwrap();
            let bytes = &input[colon + 1..colon + 1 + length];
            BencodeValue::Bytes(bytes.to_vec())
        }

        _ => todo!(), // b'l'        => // parse list
                      // b'd'        => // parse dict
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

    #[test]
    fn test_string_simple() {
        let result = decode(b"4:spam");
        assert_eq!(result, BencodeValue::Bytes(b"spam".to_vec()));
    }

    #[test]
    fn test_string_empty() {
        let result = decode(b"0:");
        assert_eq!(result, BencodeValue::Bytes(vec![]));
    }

    #[test]
    fn test_string_long_length() {
        let result = decode(b"10:helloworld");
        assert_eq!(result, BencodeValue::Bytes(b"helloworld".to_vec()));
    }

    #[test]
    fn test_string_binary() {
        // raw bytes that aren't valid UTF-8
        let input = b"3:\xFF\xFE\xFD";
        let result = decode(input);
        assert_eq!(result, BencodeValue::Bytes(vec![0xFF, 0xFE, 0xFD]));
    }
}
