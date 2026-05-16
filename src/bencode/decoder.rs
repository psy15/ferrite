#[derive(Debug, PartialEq, Clone)]
pub enum BencodeValue {
    Integer(i64),
    Bytes(Vec<u8>),
    List(Vec<BencodeValue>),
    Dict(Vec<(Vec<u8>, BencodeValue)>), // ordered, not HashMap
}

pub fn decode(input: &[u8]) -> BencodeValue {
    let (value, _) = parse(input);
    value
}

fn parse(input: &[u8]) -> (BencodeValue, usize) {
    // read bytes, return one of the 4 variants
    match input[0] {
        b'i' => {
            let end_pos = input.iter().position(|&byte| byte == b'e').unwrap();
            let digit_bytes = &input[1..end_pos];
            let digit_str = std::str::from_utf8(digit_bytes).unwrap();
            let integer = digit_str.parse::<i64>().unwrap();
            (BencodeValue::Integer(integer), end_pos + 1)
        }

        b'0'..=b'9' => {
            let colon_pos = input.iter().position(|&byte| byte == b':').unwrap();
            let length: usize = std::str::from_utf8(&input[0..colon_pos])
                .unwrap()
                .parse()
                .unwrap();
            let content = &input[colon_pos + 1..colon_pos + 1 + length];
            (
                BencodeValue::Bytes(content.to_vec()),
                colon_pos + 1 + length,
            )
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

        // d3:cow3:moo3:pigsi42ee
        b'd' => {
            let mut dict = vec![];
            let mut pos = 1;

            loop {
                if input[pos] == b'e' {
                    break;
                }
                let (key, key_consumed) = parse(&input[pos..]);
                pos += key_consumed; // advance past the key first
                let (value, val_consumed) = parse(&input[pos..]);
                pos += val_consumed; // then advance past the value

                if let BencodeValue::Bytes(k) = key {
                    dict.push((k, value));
                }
            }

            (BencodeValue::Dict(dict), pos + 1)
        }

        _ => todo!(),
    }
}

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

    #[test]
    fn test_dict_empty() {
        let result = decode(b"de");
        assert_eq!(result, BencodeValue::Dict(vec![]));
    }

    #[test]
    fn test_dict_single_pair() {
        let result = decode(b"d3:cow3:mooe");
        assert_eq!(
            result,
            BencodeValue::Dict(vec![(
                b"cow".to_vec(),
                BencodeValue::Bytes(b"moo".to_vec())
            ),])
        );
    }

    #[test]
    fn test_dict_multiple_pairs() {
        let result = decode(b"d3:cow3:moo4:spami42ee");
        assert_eq!(
            result,
            BencodeValue::Dict(vec![
                (b"cow".to_vec(), BencodeValue::Bytes(b"moo".to_vec())),
                (b"spam".to_vec(), BencodeValue::Integer(42)),
            ])
        );
    }

    #[test]
    fn test_dict_nested() {
        let result = decode(b"d4:infod4:name3:fooee");
        assert_eq!(
            result,
            BencodeValue::Dict(vec![(
                b"info".to_vec(),
                BencodeValue::Dict(vec![(
                    b"name".to_vec(),
                    BencodeValue::Bytes(b"foo".to_vec())
                ),])
            ),])
        );
    }

    #[test]
    fn test_dict_value_is_list() {
        let result = decode(b"d4:listli1ei2eee");
        assert_eq!(
            result,
            BencodeValue::Dict(vec![(
                b"list".to_vec(),
                BencodeValue::List(vec![BencodeValue::Integer(1), BencodeValue::Integer(2),])
            ),])
        );
    }

    #[test]
    fn test_real_torrent() {
        let bytes = std::fs::read("tests/fixtures/ubuntu.torrent").unwrap();
        let result = decode(&bytes);
        assert!(matches!(result, BencodeValue::Dict(_)));
    }
}
