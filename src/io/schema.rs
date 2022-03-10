use std::collections::HashMap;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DBSchema {
    pub db_name: String,
    pub db_file: String,
    pub version: usize,
    #[serde(default)]
    pub maps: HashMap<String, FileMap>,
    #[serde(default)]
    pub tables: HashMap<String, Table>,
    #[serde(default)]
    pub feeds: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileMap {
    #[serde(default)]
    pub to_table: Vec<ToTable>,
    #[serde(default)]
    pub to_keyvalue: Vec<String>,
    #[serde(default)]
    pub to_json_table: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToTable {
    pub node: String,
    pub table: String,
    pub key_col: Option<String>,
    pub val_col: Option<String>,
    pub import_cols: Option<Vec<String>>,
    pub replaces: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Table {
    pub cols: Vec<(String, String)>,
    pub indexes: Vec<String>,
    pub schema_changed: usize,
}

impl Table {
    pub fn to_query(&self, table_name: &str) -> String {
        format!(
            "CREATE TABLE {} ({})",
            table_name,
            self.cols
                .iter()
                .format_with(", ", |(col, def), f| f(&format_args!("{} {}", col, def))),
        )
    }
}
