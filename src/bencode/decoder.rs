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
    let (value, _) = parse(input);
    value
}

pub fn parse(input: &[u8]) -> (BencodeValue, usize) {
    // read bytes, return one of the 4 variants
    match input[0] {
        b'i' => {
            let end = input.iter().position(|&b| b == b'e').unwrap();
            let bytes = &input[1..end];
            let string = std::str::from_utf8(bytes).unwrap();
            let n = string.parse::<i64>().unwrap();
            (BencodeValue::Integer(n), end + 1)
        }

        b'0'..=b'9' => {
            let colon = input.iter().position(|&b| b == b':').unwrap();
            let length: usize = std::str::from_utf8(&input[0..colon])
                .unwrap()
                .parse()
                .unwrap();
            let bytes = &input[colon + 1..colon + 1 + length];
            (BencodeValue::Bytes(bytes.to_vec()), colon + 1 + length)
        }

        // [l] [i] [4] [2] [e] [4] [:] [s] [p] [a] [m] [e]
        //  0   1   2   3   4   5   6   7   8   9  10  11
        b'l' => {
            let mut items = vec![];
            let mut pos = 1;

            loop {
                if input[pos] == b'e' {
                    break;
                }
                let (value, consumed) = parse(&input[pos..]);
                items.push(value);
                pos += consumed;
            }

            (BencodeValue::List(items), pos + 1)
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

    #[test]
    fn test_list_empty() {
        let result = decode(b"le");
        assert_eq!(result, BencodeValue::List(vec![]));
    }

    #[test]
    fn test_list_of_integers() {
        let result = decode(b"li42ei-3ee");
        assert_eq!(
            result,
            BencodeValue::List(vec![BencodeValue::Integer(42), BencodeValue::Integer(-3),])
        );
    }

    #[test]
    fn test_list_of_strings() {
        let result = decode(b"l4:spam3:cowe");
        assert_eq!(
            result,
            BencodeValue::List(vec![
                BencodeValue::Bytes(b"spam".to_vec()),
                BencodeValue::Bytes(b"cow".to_vec()),
            ])
        );
    }

    #[test]
    fn test_list_nested() {
        let result = decode(b"lli42eee");
        assert_eq!(
            result,
            BencodeValue::List(vec![BencodeValue::List(vec![BencodeValue::Integer(42),]),])
        );
    }

    #[test]
    fn test_list_mixed() {
        let result = decode(b"li42e4:spame");
        assert_eq!(
            result,
            BencodeValue::List(vec![
                BencodeValue::Integer(42),
                BencodeValue::Bytes(b"spam".to_vec()),
            ])
        );
    }
}
