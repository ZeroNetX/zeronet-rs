// functionality for decoding bencoded byte strings
#![allow(dead_code)]

use super::Item;

use std::collections::BTreeMap;

fn parse_int(str: &mut Vec<u8>) -> usize {
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
    int_string.parse::<usize>().unwrap()
}

fn parse_str(str: &mut Vec<u8>) -> Vec<u8> {
    let mut int_len: usize = 0;
    let mut int_string: String = String::new();
    for c in str.iter() {
        int_len += 1;
        if *c == b':' {
            break;
        }
        int_string.push(*c as char);
    }
    let len: usize = int_string.parse::<usize>().unwrap();
    str.drain(0..int_len);

    let s = str[..len].to_vec();
    let mut copy = str[len..].to_vec();
    str.clear();
    str.append(&mut copy);
    s
}

fn parse_list(str: &mut Vec<u8>) -> Vec<Item> {
    str.drain(0..1);
    let mut list: Vec<Item> = Vec::<Item>::new();
    loop {
        match *str.get(0).unwrap() as char {
            'i' => list.push(Item::Int(parse_int(str))),
            'l' => list.push(Item::List(parse_list(str))),
            'd' => list.push(Item::Dict(parse_dict(str))),
            '0'..='9' => list.push(Item::String(parse_str(str))),
            'e' => break,
            _ => unreachable!(),
        }
    }
    str.drain(0..1);
    list
}

fn parse_dict(str: &mut Vec<u8>) -> BTreeMap<Vec<u8>, Item> {
    str.drain(0..1);
    let mut dict: BTreeMap<Vec<u8>, Item> = BTreeMap::new();
    loop {
        if *str.get(0).unwrap() == b'e' {
            break;
        }
        let s = parse_str(str);
        match *str.get(0).unwrap() as char {
            'i' => dict.insert(s, Item::Int(parse_int(str))),
            'l' => dict.insert(s, Item::List(parse_list(str))),
            'd' => dict.insert(s, Item::Dict(parse_dict(str))),
            '0'..='9' => dict.insert(s, Item::String(parse_str(str))),
            _ => unreachable!(),
        };
    }
    str.drain(0..1);
    dict
}

pub fn parse(str: &mut Vec<u8>) -> Vec<Item> {
    let mut tree: Vec<Item> = Vec::<Item>::new();
    while let Some(c) = str.get(0) {
        match *c {
            b'i' => tree.push(Item::Int(parse_int(str))),
            b'l' => tree.push(Item::List(parse_list(str))),
            b'd' => tree.push(Item::Dict(parse_dict(str))),
            b'0'..=b'9' => tree.push(Item::String(parse_str(str))),
            _ => break,
        }
    }
    tree
}
