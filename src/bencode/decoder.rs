use crate::FeriteError;

#[derive(Debug, PartialEq, Clone)]
pub enum BencodeValue {
    Integer(i64),
    Bytes(Vec<u8>),
    List(Vec<BencodeValue>),
    Dict(Vec<(Vec<u8>, BencodeValue)>), // ordered, not HashMap
}

pub fn decode(input: &[u8]) -> crate::Result<BencodeValue> {
    let (value, _) = parse(input)?;
    Ok(value)
}

fn parse(input: &[u8]) -> crate::Result<(BencodeValue, usize)> {
    // read bytes, return one of the 4 variants
    let first = input
        .first()
        .ok_or(FeriteError::Bencode("empty input".to_string()))?;
    match first {
        b'i' => {
            let end_pos = input
                .iter()
                .position(|&byte| byte == b'e')
                .ok_or(FeriteError::Bencode("missing closing e".to_string()))?;
            let digit_bytes = &input[1..end_pos];
            let digit_str = std::str::from_utf8(digit_bytes)
                .map_err(|e| FeriteError::Bencode(e.to_string()))?;
            let integer = digit_str
                .parse::<i64>()
                .map_err(|e: std::num::ParseIntError| FeriteError::Bencode(e.to_string()))?;
            Ok((BencodeValue::Integer(integer), end_pos + 1))
        }

        b'0'..=b'9' => {
            let colon_pos = input
                .iter()
                .position(|&byte| byte == b':')
                .ok_or(FeriteError::Bencode("missing colon in string".to_string()))?;
            let length: usize = std::str::from_utf8(&input[0..colon_pos])
                .map_err(|e| FeriteError::Bencode(e.to_string()))?
                .parse()
                .map_err(|e: std::num::ParseIntError| FeriteError::Bencode(e.to_string()))?;
            let content = &input[colon_pos + 1..colon_pos + 1 + length];
            Ok((
                BencodeValue::Bytes(content.to_vec()),
                colon_pos + 1 + length,
            ))
        }

        b'l' => {
            let mut items = vec![];
            let mut pos = 1;

            loop {
                if pos >= input.len() {
                    return Err(FeriteError::Bencode("missing closing e".to_string()));
                }
                if input.get(pos) == Some(&b'e') {
                    break;
                }
                let (value, consumed) = parse(&input[pos..])?;
                items.push(value);
                pos += consumed;
            }

            Ok((BencodeValue::List(items), pos + 1))
        }

        // d3:cow3:moo3:pigsi42ee
        b'd' => {
            let mut dict = vec![];
            let mut pos = 1;

            loop {
                if pos >= input.len() {
                    return Err(FeriteError::Bencode("missing closing e".to_string()));
                }
                if input.get(pos) == Some(&b'e') {
                    break;
                }
                let (key, key_consumed) = parse(&input[pos..])?;
                pos += key_consumed; // advance past the key first
                let (value, val_consumed) = parse(&input[pos..])?;
                pos += val_consumed; // then advance past the value

                if let BencodeValue::Bytes(k) = key {
                    dict.push((k, value));
                }
            }

            Ok((BencodeValue::Dict(dict), pos + 1))
        }

        _ => Err(FeriteError::Bencode(format!(
            "unexpected byte: {}",
            input[0]
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer() {
        let input = b"i42e";
        let result: BencodeValue = decode(input).unwrap();
        assert_eq!(result, BencodeValue::Integer(42));
    }

    #[test]
    fn test_negative_integer() {
        let result = decode(b"i-3e").unwrap();
        assert_eq!(result, BencodeValue::Integer(-3));
    }

    #[test]
    fn test_zero() {
        let result = decode(b"i0e").unwrap();
        assert_eq!(result, BencodeValue::Integer(0));
    }

    #[test]
    fn test_string_simple() {
        let result = decode(b"4:spam").unwrap();
        assert_eq!(result, BencodeValue::Bytes(b"spam".to_vec()));
    }

    #[test]
    fn test_string_empty() {
        let result = decode(b"0:").unwrap();
        assert_eq!(result, BencodeValue::Bytes(vec![]));
    }

    #[test]
    fn test_string_long_length() {
        let result = decode(b"10:helloworld").unwrap();
        assert_eq!(result, BencodeValue::Bytes(b"helloworld".to_vec()));
    }

    #[test]
    fn test_string_binary() {
        // raw bytes that aren't valid UTF-8
        let input = b"3:\xFF\xFE\xFD";
        let result = decode(input).unwrap();
        assert_eq!(result, BencodeValue::Bytes(vec![0xFF, 0xFE, 0xFD]));
    }

    #[test]
    fn test_list_empty() {
        let result = decode(b"le").unwrap();
        assert_eq!(result, BencodeValue::List(vec![]));
    }

    #[test]
    fn test_list_of_integers() {
        let result = decode(b"li42ei-3ee").unwrap();
        assert_eq!(
            result,
            BencodeValue::List(vec![BencodeValue::Integer(42), BencodeValue::Integer(-3),])
        );
    }

    #[test]
    fn test_list_of_strings() {
        let result = decode(b"l4:spam3:cowe").unwrap();
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
        let result = decode(b"lli42eee").unwrap();
        assert_eq!(
            result,
            BencodeValue::List(vec![BencodeValue::List(vec![BencodeValue::Integer(42),]),])
        );
    }

    #[test]
    fn test_list_mixed() {
        let result = decode(b"li42e4:spame").unwrap();
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
        let result = decode(b"de").unwrap();
        assert_eq!(result, BencodeValue::Dict(vec![]));
    }

    #[test]
    fn test_dict_single_pair() {
        let result = decode(b"d3:cow3:mooe").unwrap();
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
        let result = decode(b"d3:cow3:moo4:spami42ee").unwrap();
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
        let result = decode(b"d4:infod4:name3:fooee").unwrap();
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
        let result = decode(b"d4:listli1ei2eee").unwrap();
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
        let result = decode(&bytes).unwrap();
        assert!(matches!(result, BencodeValue::Dict(_)));
    }

    #[test]
    fn test_invalid_input_returns_error() {
        let result = decode(b"x42e");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_input_returns_error() {
        let result = decode(b"");
        assert!(result.is_err());
    }
}
