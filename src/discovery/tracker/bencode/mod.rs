// bencode subfolder and item enum implemenation
#![allow(dead_code)]

pub mod decode;
pub mod encode;

use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub enum Item {
    Int(usize),
    String(Vec<u8>),
    List(Vec<Item>),
    Dict(BTreeMap<Vec<u8>, Item>),
}

impl Item {
    #[allow(dead_code)]
    pub fn get_int(&self) -> usize {
        let int = match &self {
            Item::Int(int) => int,
            _ => unreachable!(),
        };
        *int
    }
    #[allow(dead_code)]
    pub fn get_str(&self) -> Vec<u8> {
        let str = match &self {
            Item::String(str) => str,
            _ => unreachable!(),
        };
        str.clone()
    }
    #[allow(dead_code)]
    pub fn get_list(&self) -> Vec<Item> {
        let list = match &self {
            Item::List(list) => list,
            _ => unreachable!(),
        };
        list.clone()
    }
    #[allow(dead_code)]
    pub fn get_dict(&self) -> BTreeMap<Vec<u8>, Item> {
        let dict = match &self {
            Item::Dict(dict) => dict,
            _ => unreachable!(),
        };
        dict.clone()
    }
}
