// functionality for decoding bencoded byte strings
#![allow(dead_code)]

use super::{Error, Item};

use std::collections::BTreeMap;

fn parse_int(str: &mut Vec<u8>) -> Result<usize, Error> {
    let mut len: usize = 0;
    let mut int_string: String = String::new();
    for c in str.iter() {
        len += 1;
        if *c == b'i' {
            continue;
        }
        if *c == b'e' {
            break;
        }
        int_string.push(*c as char);
    }
    str.drain(0..len);
    let res = int_string.parse::<usize>();
    if res.is_err() {
        Err(Error::InvalidInt)
    } else {
        Ok(res.unwrap())
    }
}

fn parse_str(str: &mut Vec<u8>) -> Result<Vec<u8>, Error> {
    let mut int_len: usize = 0;
    let mut int_string: String = String::new();
    for c in str.iter() {
        int_len += 1;
        if *c == b':' {
            break;
        }
        int_string.push(*c as char);
    }
    let len = int_string.parse::<usize>();
    if len.is_err() {
        return Err(Error::InvalidString);
    }
    let len = len.unwrap();
    str.drain(0..int_len);

    let s = str[..len].to_vec();
    let mut copy = str[len..].to_vec();
    str.clear();
    str.append(&mut copy);
    Ok(s)
}

fn parse_list(str: &mut Vec<u8>) -> Result<Vec<Item>, Error> {
    str.drain(0..1);
    let mut list: Vec<Item> = Vec::<Item>::new();
    loop {
        match *str.get(0).unwrap() as char {
            'i' => list.push(Item::Int(parse_int(str)?)),
            'l' => list.push(Item::List(parse_list(str)?)),
            'd' => list.push(Item::Dict(parse_dict(str)?)),
            '0'..='9' => list.push(Item::String(parse_str(str)?)),
            'e' => break,
            _ => unreachable!(),
        }
    }
    str.drain(0..1);
    Ok(list)
}

fn parse_dict(str: &mut Vec<u8>) -> Result<BTreeMap<Vec<u8>, Item>, Error> {
    str.drain(0..1);
    let mut dict: BTreeMap<Vec<u8>, Item> = BTreeMap::new();
    loop {
        if *str.get(0).unwrap() == b'e' {
            break;
        }
        let s = parse_str(str)?;
        match *str.get(0).unwrap() as char {
            'i' => dict.insert(s, Item::Int(parse_int(str)?)),
            'l' => dict.insert(s, Item::List(parse_list(str)?)),
            'd' => dict.insert(s, Item::Dict(parse_dict(str)?)),
            '0'..='9' => dict.insert(s, Item::String(parse_str(str)?)),
            _ => unreachable!(),
        };
    }
    str.drain(0..1);
    Ok(dict)
}

pub fn parse(str: &mut Vec<u8>) -> Result<Vec<Item>, Error> {
    let mut tree: Vec<Item> = Vec::<Item>::new();
    while let Some(c) = str.get(0) {
        match *c {
            b'i' => tree.push(Item::Int(parse_int(str)?)),
            b'l' => tree.push(Item::List(parse_list(str)?)),
            b'd' => tree.push(Item::Dict(parse_dict(str)?)),
            b'0'..=b'9' => tree.push(Item::String(parse_str(str)?)),
            _ => break,
        }
    }
    Ok(tree)
}
