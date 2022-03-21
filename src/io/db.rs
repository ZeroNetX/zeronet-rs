use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use async_recursion::async_recursion;
use log::debug;
use regex::Regex;
use rusqlite::{params, Connection};
use serde_json::Value;
use tokio::fs;

use crate::{core::schema::*, environment::ENV};
pub struct DbManager {
    db: HashMap<String, Connection>,
    pub schema: HashMap<String, DBSchema>,
}

impl Default for DbManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DbManager {
    pub fn new() -> DbManager {
        DbManager {
            db: HashMap::new(),
            schema: HashMap::default(),
        }
    }

    pub fn has_schema(&self, site: &str) -> (bool, Option<PathBuf>) {
        let data_path = ENV.data_path.join(site);
        let schema_path = data_path.join("dbschema.json");
        if schema_path.is_file() {
            (true, Some(schema_path))
        } else {
            (false, None)
        }
    }

    pub fn connect_db(&mut self, site: &str) {
        let db_path = self.schema[site].db_file.clone();
        let conn =
            Connection::open(ENV.data_path.join(site).join(db_path).to_str().unwrap()).unwrap();
        self.db.insert(site.into(), conn);
    }

    pub fn load_schema(&mut self, site: &str) -> Option<DBSchema> {
        let (_, schema) = self.has_schema(site);
        if let Some(path) = schema {
            let schema_str = std::fs::read_to_string(&path).unwrap();
            let mut schema: DBSchema = serde_json::from_str(&schema_str).unwrap();
            let version = schema.version;
            let mut table_names = vec![];
            for table_name in schema.tables.keys() {
                table_names.push(table_name.clone());
            }
            // println!("{:?}", table_names);
            if table_names.contains(&"json".to_string()) {
                debug!("Json tables specified in tables");
            } else {
                let json_table = Self::def_json_table(version);
                schema.tables.insert("json".to_string(), json_table);
            }
            let key_value_table = Self::def_keyvalue_table();
            schema
                .tables
                .insert("keyvalue".to_string(), key_value_table);
            self.schema.insert(site.into(), schema.clone());
            return Some(schema);
        }
        None
    }

    pub fn create_tables(&mut self, site: &str) {
        let tables = self.schema[site].tables.clone();
        let conn = self.get_db(site).unwrap();
        tables.keys().for_each(|table_name| {
            if table_name == "json" {
                //Note: Required because other tables depend on json table, it needs to droped last.
                return;
            }
            let _res = conn.execute(&format!("DROP TABLE {}", table_name), []);
        });
        let _res = conn.execute("DROP TABLE json", []);
        for (table_name, table) in tables {
            let query = table.to_query(&table_name);
            conn.execute(&query, params![]).unwrap();
            let indexes = table.indexes;
            indexes.into_iter().for_each(|i| {
                conn.execute(&i, params![]).unwrap();
            });
        }
    }

    pub fn def_keyvalue_table() -> Table {
        Table {
            cols: vec![
                (
                    "keyvalue_id".to_string(),
                    "INTEGER PRIMARY KEY AUTOINCREMENT".to_string(),
                ),
                ("key".to_string(), "TEXT".to_string()),
                ("value".to_string(), "INTEGER".to_string()),
                ("json_id".to_string(), "INTEGER".to_string()),
            ],
            indexes: vec!["CREATE UNIQUE INDEX key_id ON keyvalue(json_id, key)".to_string()],
            schema_changed: 1,
        }
    }

    pub fn def_json_table(version: usize) -> Table {
        match version {
            1 => Table {
                cols: vec![
                    (
                        "json_id".to_string(),
                        "INTEGER PRIMARY KEY AUTOINCREMENT".to_string(),
                    ),
                    ("path".to_string(), "VARCHAR(255)".to_string()),
                ],
                indexes: vec!["CREATE UNIQUE INDEX path ON json(path)".to_string()],
                schema_changed: 1,
            },
            2 => Table {
                cols: vec![
                    (
                        "json_id".to_string(),
                        "INTEGER PRIMARY KEY AUTOINCREMENT".to_string(),
                    ),
                    ("directory".to_string(), "VARCHAR(255)".to_string()),
                    ("file_name".to_string(), "VARCHAR(255)".to_string()),
                ],
                indexes: vec!["CREATE UNIQUE INDEX path ON json(directory, file_name)".to_string()],
                schema_changed: 1,
            },
            3 => Table {
                cols: vec![
                    (
                        "json_id".to_string(),
                        "INTEGER PRIMARY KEY AUTOINCREMENT".to_string(),
                    ),
                    ("site".to_string(), "VARCHAR(255)".to_string()),
                    ("directory".to_string(), "VARCHAR(255)".to_string()),
                    ("file_name".to_string(), "VARCHAR(255)".to_string()),
                ],
                indexes: vec![
                    "CREATE UNIQUE INDEX path ON json(directory, site, file_name)".to_string(),
                ],
                schema_changed: 1,
            },
            _ => unreachable!(),
        }
    }

    pub fn get_db(&mut self, site_name: &str) -> Option<&mut Connection> {
        self.db.get_mut(site_name)
    }
}

impl DbManager {
    pub async fn load_data(&mut self, site: &str) {
        let schema = self.schema[site].clone();
        let version = schema.version;
        let maps = schema.maps;
        let db_path: PathBuf = schema.db_file.into();
        let data_path = ENV.data_path.join(site);
        let db_path = data_path.join(db_path);
        let db_dir = db_path.parent().unwrap();
        let paths = Self::load_entries(db_dir, None).await;
        // println!("{:?}", paths);
        let mut regexes = vec![];
        for regex_str in maps.keys() {
            let regex = Regex::new(regex_str).unwrap();
            regexes.push((regex_str, regex));
        }
        let mut handlers = vec![];
        for path in paths {
            for (regex_str, regex) in &regexes {
                let matched = regex.is_match(&path);
                if matched {
                    handlers.push((regex_str.to_string(), path.to_string()));
                }
            }
        }
        let _conn = self.get_db(site).unwrap();
        //31 //.iter().skip(15).take(1).collect_vec()
        for (regex_str, path_str) in handlers {
            let map = (maps[&regex_str]).clone();
            let path = Path::new(&db_dir).join(&path_str);
            let content = std::fs::read_to_string(&path).unwrap();
            let json_content: HashMap<String, Value> = serde_json::from_str(&content).unwrap();
            let has_custom_table = !&map.to_json_table.is_empty();

            let json_id: i64 = Self::handle_json_table(
                version,
                has_custom_table,
                &path_str,
                site,
                &map.to_json_table,
                &json_content,
                _conn,
            );

            Self::load_key_value_table(&map.to_keyvalue, json_id, &json_content, _conn);
            Self::handle_to_table_map(&map.to_table, json_id, &json_content, _conn);
        }
    }

    fn handle_json_table(
        version: usize,
        has_custom_table: bool,
        path_str: &str,
        site: &str,
        to_json_table: &[String],
        json_content: &HashMap<String, Value>,
        _conn: &mut Connection,
    ) -> i64 {
        // println!("{:?}", json_content);
        let (mut json_statement, mut values, select_statement) = match version {
            1 => (
                "path".to_string(),
                format!("'{}'", path_str),
                format!("path = '{}'", path_str),
            ),
            2 => {
                let mut v = path_str.split('/').collect::<Vec<&str>>();
                let (directory, file_name) = if v.len() == 2 {
                    (v[0].to_owned(), v[1].to_owned())
                } else {
                    let file_name = v.last().unwrap().to_string();
                    v.pop();
                    let directory = v.join("/");
                    (directory, file_name)
                };
                (
                    "directory, file_name".to_string(),
                    format!("'{}', '{}'", directory, file_name),
                    format!(
                        "directory = '{}' AND file_name = '{}'",
                        directory, file_name
                    ),
                )
            }
            3 => {
                let mut v = path_str.split('/').collect::<Vec<&str>>();
                let (directory, file_name) = if v.len() == 2 {
                    (v[0].to_owned(), v[1].to_owned())
                } else {
                    let file_name = v.last().unwrap().to_string();
                    v.pop();
                    let directory = v.join("/");
                    (directory, file_name)
                };
                (
                    "site, directory, file_name".to_string(),
                    format!("'{}', '{}', '{}'", site, directory, file_name),
                    format!(
                        "site = '{}' AND directory = '{}' AND file_name = '{}'",
                        site, directory, file_name
                    ),
                )
            }
            _ => unreachable!(),
        };
        if has_custom_table {
            for table in to_json_table {
                let key = format!(", {}", table);
                json_statement.push_str(&key);
                let value = json_content.get(&*table).unwrap();
                if let Value::String(value) = value {
                    let value = value.replace('\'', "''");
                    values.push_str(&format!(", '{}'", value));
                } else if let Value::Number(value) = value {
                    values.push_str(&format!(", {}", value));
                }
            }
        }
        let json_statement = format!("INSERT INTO json ({}) VALUES ({})", json_statement, values);
        let select_statement = format!("SELECT json_id FROM json WHERE ({})", select_statement);
        let _result = _conn.execute(&json_statement, params![]);
        let mut stmt = (&*_conn).prepare(&select_statement).unwrap();
        let mut rows = stmt.query([]).unwrap();
        let a = rows.next().unwrap();
        a.unwrap().get(0).unwrap()
    }

    fn handle_to_table_map(
        to_table: &[ToTable],
        json_id: i64,
        content: &HashMap<String, Value>,
        _conn: &Connection,
    ) {
        for to_table in to_table {
            let node = to_table.node.clone();
            let table = to_table.table.clone();
            let key_col = to_table.key_col.clone();
            let value_col = to_table.val_col.clone();
            let import_col = to_table.import_cols.clone();
            let replaces = to_table.replaces.clone();
            let value = &content[node.as_str()].clone();
            if let Value::Array(a) = value {
                if a.is_empty() {
                    continue;
                }
            }
            if let Value::Object(a) = value {
                if a.is_empty() {
                    continue;
                }
            }
            let mut import_cols = vec![];
            let use_import_cols = import_col.is_some();
            if let Some(cols) = import_col {
                for col in cols {
                    import_cols.push(col);
                }
            }
            let mut replacement_cols = vec![];
            let mut replacements = vec![];
            if let Some(replaces) = replaces {
                for (col_name, replacements_map) in replaces {
                    for (key, value) in replacements_map {
                        replacement_cols.push(col_name.clone());
                        replacements.push((key.clone(), value.clone()));
                    }
                }
            }
            //TODO: Simplify below code
            if let Value::Array(object) = value {
                for value in object {
                    let mut column_keys = vec![];
                    let mut values = vec![];
                    if let Value::Object(obj) = value {
                        for (key, value) in obj {
                            if use_import_cols && !import_cols.contains(key) {
                                continue;
                            }
                            let key = (&*key).to_string();
                            let mut need_replacement = false;
                            let mut replacement_idx = 255;
                            if replacement_cols.contains(&key) {
                                need_replacement = true;
                                replacement_idx =
                                    replacement_cols.iter().position(|x| x == &key).unwrap();
                            }
                            column_keys.push(key);
                            if let Value::String(value) = value {
                                //TODO!: Do we need to escape the "(" and ")" ?
                                let mut value = value.replace('\'', "''");
                                if need_replacement {
                                    let rep_vec = replacements.get(replacement_idx).unwrap();
                                    value = value.replace(&rep_vec.0, &rep_vec.1);
                                }
                                values.push(format!("'{}'", value));
                            } else if let Value::Number(value) = value {
                                values.push(format!("{}", value));
                            }
                        }
                    } else {
                        unimplemented!("Please file a bug report");
                    }
                    column_keys.push("json_id".to_owned());
                    values.push(format!("{}", json_id));
                    let stmt = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        table,
                        column_keys.join(", "),
                        values.join(", ")
                    );
                    //TODO!: Handle Result
                    let _res = _conn.execute(&stmt, []);
                }
            } else if let Value::Object(obj) = value {
                for (key, value) in obj {
                    let mut column_keys = vec![];
                    let mut values = vec![];
                    // let mut stmt = format!("INSERT INTO {} (", table);
                    // let mut values = format!("VALUES (");
                    if let Some(key_column_name) = &key_col {
                        if let Some(column_name) = &value_col {
                            let key_col = key_column_name.clone();
                            let mut value_str = key.clone();
                            let mut need_replacement = false;
                            let mut replacement_idx = 255;
                            if replacement_cols.contains(&key_col) {
                                need_replacement = true;
                                replacement_idx =
                                    replacement_cols.iter().position(|x| x == &key_col).unwrap();
                            }
                            column_keys.push(key_col);
                            if need_replacement {
                                let rep_vec = replacements.get(replacement_idx).unwrap();
                                value_str = value_str.replace(&rep_vec.0, &rep_vec.1);
                            }
                            values.push(format!("'{}'", value_str));
                            let key_col = column_name.clone();
                            let mut need_replacement = false;
                            let mut replacement_idx = 255;
                            if replacement_cols.contains(&key_col) {
                                need_replacement = true;
                                replacement_idx =
                                    replacement_cols.iter().position(|x| x == &key_col).unwrap();
                            }
                            column_keys.push(key_col);
                            if let Value::String(value) = value {
                                //TODO!: Do we need to escape the "(" and ")" ?
                                let mut value = value.replace('\'', "''");
                                if need_replacement {
                                    let rep_vec = replacements.get(replacement_idx).unwrap();
                                    value = value.replace(&rep_vec.0, &rep_vec.1);
                                }
                                values.push(format!("'{}'", value));
                            } else if let Value::Number(value) = value {
                                values.push(format!("{}", value));
                            }
                        } else {
                            let key_col = key_column_name.clone();
                            let mut value_str = key.clone();
                            let mut need_replacement = false;
                            let mut replacement_idx = 255;
                            if replacement_cols.contains(&key_col) {
                                need_replacement = true;
                                replacement_idx =
                                    replacement_cols.iter().position(|x| x == &key_col).unwrap();
                            }
                            column_keys.push(key_col);
                            if need_replacement {
                                let rep_vec = replacements.get(replacement_idx).unwrap();
                                value_str = value_str.replace(&rep_vec.0, &rep_vec.1);
                            }
                            values.push(format!("'{}'", value_str));
                            if let Value::Object(value) = value {
                                for (key_col, value) in value {
                                    if use_import_cols && !import_cols.contains(key_col) {
                                        continue;
                                    }
                                    let mut need_replacement = false;
                                    let mut replacement_idx = 255;
                                    if replacement_cols.contains(key_col) {
                                        need_replacement = true;
                                        replacement_idx = replacement_cols
                                            .iter()
                                            .position(|x| x == key_col)
                                            .unwrap();
                                    }
                                    column_keys.push(key_col.to_string());
                                    if let Value::String(value) = value {
                                        //TODO!: Do we need to escape the "(" and ")" ?
                                        let mut value = value.replace('\'', "''");
                                        if need_replacement {
                                            let rep_vec =
                                                replacements.get(replacement_idx).unwrap();
                                            value = value.replace(&rep_vec.0, &rep_vec.1);
                                        }
                                        values.push(format!("'{}'", value));
                                    } else if let Value::Number(value) = value {
                                        values.push(format!("{}", value));
                                    }
                                }
                            } else if let Value::Array(value) = value {
                                for value in value {
                                    if let Value::Object(value) = value {
                                        for (key_col, value) in value {
                                            if use_import_cols && !import_cols.contains(key_col) {
                                                continue;
                                            }
                                            let mut need_replacement = false;
                                            let mut replacement_idx = 255;
                                            if replacement_cols.contains(key_col) {
                                                need_replacement = true;
                                                replacement_idx = replacement_cols
                                                    .iter()
                                                    .position(|x| x == key_col)
                                                    .unwrap();
                                            }
                                            column_keys.push(key_col.to_string());
                                            if let Value::String(value) = value {
                                                //TODO!: Do we need to escape the "(" and ")" ?
                                                let mut value = value.replace('\'', "''");
                                                if need_replacement {
                                                    let rep_vec =
                                                        replacements.get(replacement_idx).unwrap();
                                                    value = value.replace(&rep_vec.0, &rep_vec.1);
                                                }
                                                values.push(format!("'{}'", value));
                                            } else if let Value::Number(value) = value {
                                                values.push(format!("{}", value));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        let mut need_replacement = false;
                        let mut replacement_idx = 255;
                        if replacement_cols.contains(key) {
                            need_replacement = true;
                            replacement_idx =
                                replacement_cols.iter().position(|x| x == key).unwrap();
                        }
                        column_keys.push(key.to_string());
                        if let Value::String(value) = value {
                            //TODO!: Do we need to escape the "(" and ")" ?
                            let mut value = value.replace('\'', "''");
                            if need_replacement {
                                let rep_vec = replacements.get(replacement_idx).unwrap();
                                value = value.replace(&rep_vec.0, &rep_vec.1);
                            }
                            values.push(format!("'{}'", value));
                        } else if let Value::Number(value) = value {
                            values.push(format!("{}", value));
                        }
                    }
                    column_keys.push("json_id".to_string());
                    values.push(format!("{}", json_id));
                    let stmt = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        table,
                        column_keys.join(", "),
                        values.join(", ")
                    );
                    //TODO!: Handle Result
                    let _res = _conn.execute(&stmt, []);
                    // println!("{:?}", _res);
                }
            } else {
                unreachable!("Please File a Bug Request");
            }
        }
    }

    fn load_key_value_table(
        keyvalue: &[String],
        json_id: i64,
        content: &HashMap<String, Value>,
        _conn: &Connection,
    ) {
        for key in keyvalue {
            let value: &Value = &content[key];
            if let Some(value) = value.as_u64() {
                let query = format!(
                    "INSERT INTO keyvalue (key, value, json_id) VALUES ('{}', {}, {})",
                    key, value, json_id
                );
                //TODO!: Handle Result
                let _res = _conn.execute(&query, params![]);
            } else if let Some(value) = value.as_str() {
                let query = format!(
                    "INSERT INTO keyvalue (key, value, json_id) VALUES ('{}', '{}', {})",
                    key, value, json_id
                );
                //TODO!: Handle Result
                let _res = _conn.execute(&query, params![]);
            }
        }
    }

    #[async_recursion]
    pub async fn load_entries<'a>(db_dir: &'a Path, inner_path: Option<&'a Path>) -> Vec<String> {
        let path = if let Some(path) = inner_path {
            path
        } else {
            db_dir
        };
        let mut paths = vec![];
        let mut files = fs::read_dir(path).await.unwrap();
        while let Ok(Some(entry)) = files.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                let path = path.display().to_string();
                // let path =
                let path = &path[db_dir.to_str().unwrap().len() + 1..];
                // println!("Loading file: {}", path);
                let path = path.replace('\\', "/");
                paths.push(path);
            } else if path.is_dir() {
                let sub_paths = Self::load_entries(db_dir, Some(&path)).await;
                paths.extend(sub_paths);
            }
        }
        paths
    }
}
