use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
};

use async_recursion::async_recursion;
use log::*;
use regex::Regex;
use rusqlite::{params, Connection};
use serde_json::Value;
use tokio::fs;

use crate::{
    core::{error::Error, schema::*},
    environment::ENV,
};
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

    pub fn connect_db_from_path(path: &Path) -> Result<Connection, Error> {
        Connection::open(path).map_err(|e| Error::Err(e.to_string()))
    }

    pub fn connect_db(&mut self, site: &str) -> Result<(), Error> {
        let db_path = self.schema[site].db_file.clone();
        let site_db_path = ENV.data_path.join(site).join(db_path);
        let conn = Self::connect_db_from_path(&site_db_path)?;
        self.insert_connection(site, conn);
        Ok(())
    }

    pub fn load_schema_from_str(schema_str: &str) -> DBSchema {
        let mut schema: DBSchema = serde_json::from_str(schema_str).unwrap();
        let version = schema.version;
        let mut table_names = vec![];
        for table_name in schema.tables.keys() {
            table_names.push(table_name.clone());
        }
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
        schema
    }

    pub fn load_schema_from_path(path: &Path) -> DBSchema {
        let schema_str = std::fs::read_to_string(path).unwrap();
        Self::load_schema_from_str(&schema_str)
    }

    pub fn insert_schema(&mut self, site: &str, schema: DBSchema) {
        self.schema.insert(site.into(), schema);
    }

    pub fn insert_connection(&mut self, site: &str, conn: Connection) {
        self.db.insert(site.into(), conn);
    }

    pub fn load_schema(&mut self, site: &str) -> Option<DBSchema> {
        let (_, schema) = self.has_schema(site);
        if let Some(path) = schema {
            return Some(Self::load_schema_from_path(&path));
        }
        None
    }

    pub fn create_tables(&mut self, site: &str) {
        let tables = self.schema[site].tables.clone();
        let conn = self.get_db(site).unwrap();
        tables.keys().for_each(|table_name| {
            if table_name == "json" {
                //Note: Required because other tables depend on json table, it needs to be dropped last.
                return;
            }
            Self::db_exec(conn, &format!("DROP TABLE {table_name}"));
        });
        Self::db_exec(conn, "DROP TABLE json");
        let mut sorted_tables = Vec::<(String, Table)>::new();
        let mut tables = tables;
        let mut sorted = false;
        let mut initial = true;
        while !sorted {
            let need = ["keyvalue", "json"];
            if initial {
                for table_name in need {
                    let table_name = table_name.to_string();
                    sorted_tables.push((table_name.clone(), tables[&table_name].clone()));
                    tables.remove(&table_name);
                }
                initial = false;
            }
            for (table_name, table) in tables.clone() {
                let ref_tables = |query: &str| {
                    let mut parts = query.split("REFERENCES ").collect::<VecDeque<&str>>();
                    {
                        let mut refs = vec![];
                        parts.pop_front();
                        while let Some(part) = parts.pop_front() {
                            let ref_ = part.split('(').next().unwrap().replace(' ', "");
                            if !need.contains(&ref_.as_str()) {
                                refs.push(ref_);
                            }
                        }
                        refs
                    }
                };
                let refs = ref_tables(&table.to_query(&table_name));
                let keys = sorted_tables
                    .iter()
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>();
                for ref_table in refs {
                    if !keys.contains(&&ref_table) {
                        continue;
                    }
                }
                let table_name = table_name.to_string();
                if !keys.contains(&&table_name) {
                    sorted_tables.push((table_name.clone(), tables[&table_name].clone()));
                    tables.remove(&table_name);
                }
            }
            sorted = tables.is_empty();
        }
        for (table_name, table) in sorted_tables {
            let query = table.to_query(&table_name);

            Self::db_exec(conn, &query);
            if let Some(indexes) = table.indexes {
                indexes.into_iter().for_each(|i| {
                    Self::db_exec(conn, &i);
                });
            }
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
            indexes: Some(vec![
                "CREATE UNIQUE INDEX key_id ON keyvalue(json_id, key)".to_string()
            ]),
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
                indexes: Some(vec!["CREATE UNIQUE INDEX path ON json(path)".to_string()]),
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
                indexes: Some(vec![
                    "CREATE UNIQUE INDEX path ON json(directory, file_name)".to_string(),
                ]),
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
                indexes: Some(vec![
                    "CREATE UNIQUE INDEX path ON json(directory, site, file_name)".to_string(),
                ]),
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
            let content = std::fs::read_to_string(path).unwrap();
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
        conn: &mut Connection,
    ) -> i64 {
        // println!("{:?}", json_content);
        let (mut json_statement, mut values, select_statement) = match version {
            1 => (
                "path".to_string(),
                format!("'{path_str}'"),
                format!("path = '{path_str}'"),
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
                    format!("'{directory}', '{file_name}'"),
                    format!(
                        "directory = '{directory}' AND file_name = '{file_name}'"
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
                    format!("'{site}', '{directory}', '{file_name}'"),
                    format!(
                        "site = '{site}' AND directory = '{directory}' AND file_name = '{file_name}'"
                    ),
                )
            }
            _ => unreachable!(),
        };
        if has_custom_table {
            for table in to_json_table {
                let key = format!(", {table}");
                json_statement.push_str(&key);
                let value = json_content.get(table).unwrap();
                if let Value::String(value) = value {
                    let value = value.replace('\'', "''");
                    values.push_str(&format!(", '{value}'"));
                } else if let Value::Number(value) = value {
                    values.push_str(&format!(", {value}"));
                }
            }
        }
        let json_statement = format!("INSERT INTO json ({json_statement}) VALUES ({values})");
        let select_statement = format!("SELECT json_id FROM json WHERE ({select_statement})");
        Self::db_exec(conn, &json_statement);
        let mut stmt = (*conn).prepare(&select_statement).unwrap();
        let mut rows = stmt.query([]).unwrap();
        let a = rows.next().unwrap();
        a.unwrap().get(0).unwrap()
    }

    fn handle_to_table_map(
        to_table_list: &[EitherToTableType],
        json_id: i64,
        content: &HashMap<String, Value>,
        conn: &Connection,
    ) {
        for to_table in to_table_list {
            let (table, node, key_col, value_col, import_col, replaces) = match to_table {
                EitherToTableType::String(to_table) => {
                    (to_table.into(), to_table.into(), None, None, None, None)
                }
                EitherToTableType::ToTable(to_table) => {
                    let table = to_table.table.clone();
                    let node = to_table.node.clone().unwrap_or_else(|| table.clone());
                    let key_col = to_table.key_col.clone();
                    let value_col = to_table.val_col.clone();
                    let import_col = to_table.import_cols.clone();
                    let replaces = to_table.replaces.clone();
                    (table, node, key_col, value_col, import_col, replaces)
                }
            };
            let value = content.get(&node);
            if value.is_none() {
                continue;
            }

            let value = value.unwrap();

            if let Value::Array(v) = value {
                if v.is_empty() {
                    continue;
                }
            } else if let Value::Object(v) = value
                && v.is_empty() {
                    continue;
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
                            DbManager::object_handler(
                                value,
                                key,
                                &replacements,
                                &replacement_cols,
                                &mut column_keys,
                                &mut values,
                            );
                        }
                    } else {
                        unimplemented!("Please file a bug report");
                    }
                    DbManager::insert_to_table(
                        &mut column_keys,
                        &json_id,
                        &mut values,
                        &table,
                        conn,
                    );
                }
            } else if let Value::Object(obj) = value {
                for (key, value) in obj {
                    let mut column_keys = vec![];
                    let mut values = vec![];
                    // let mut stmt = format!("INSERT INTO {} (", table);
                    // let mut values = format!("VALUES (");
                    if let Some(key_column_name) = &key_col {
                        if let Some(column_name) = &value_col {
                            DbManager::object_str_handler(
                                key_column_name,
                                key.clone(),
                                &mut column_keys,
                                &replacements,
                                &replacement_cols,
                                &mut values,
                            );

                            DbManager::object_handler(
                                value,
                                column_name,
                                &replacements,
                                &replacement_cols,
                                &mut column_keys,
                                &mut values,
                            );
                        } else {
                            DbManager::object_str_handler(
                                key_column_name,
                                key.clone(),
                                &mut column_keys,
                                &replacements,
                                &replacement_cols,
                                &mut values,
                            );

                            if let Value::Object(value) = value {
                                for (key_col, value) in value {
                                    if use_import_cols && !import_cols.contains(key_col) {
                                        continue;
                                    }
                                    DbManager::object_handler(
                                        value,
                                        key_col,
                                        &replacements,
                                        &replacement_cols,
                                        &mut column_keys,
                                        &mut values,
                                    );
                                }
                            } else if let Value::Array(value) = value {
                                for value in value {
                                    if let Value::Object(value) = value {
                                        for (key_col, value) in value {
                                            if use_import_cols && !import_cols.contains(key_col) {
                                                continue;
                                            }
                                            DbManager::object_handler(
                                                value,
                                                key_col,
                                                &replacements,
                                                &replacement_cols,
                                                &mut column_keys,
                                                &mut values,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        DbManager::object_handler(
                            value,
                            key,
                            &replacements,
                            &replacement_cols,
                            &mut column_keys,
                            &mut values,
                        );
                    }

                    DbManager::insert_to_table(
                        &mut column_keys,
                        &json_id,
                        &mut values,
                        &table,
                        conn,
                    );
                }
            } else {
                unreachable!("Please File a Bug Request");
            }
        }
    }

    fn object_str_handler(
        key_col: &str,
        key: String,
        column_keys: &mut Vec<String>,
        replacements: &[(String, String)],
        replacement_cols: &[String],
        values: &mut Vec<String>,
    ) {
        let mut value_str = key;
        let replacement_idx = replacement_cols.iter().position(|x| x == key_col);
        column_keys.push(key_col.to_string());
        if let Some(replacement_idx) = replacement_idx {
            let rep_vec = replacements.get(replacement_idx).unwrap();
            value_str = value_str.replace(&rep_vec.0, &rep_vec.1);
        }
        values.push(format!("'{value_str}'"));
    }

    fn object_handler(
        value: &Value,
        key_col: &str,
        replacements: &[(String, String)],
        replacement_cols: &[String],
        column_keys: &mut Vec<String>,
        values: &mut Vec<String>,
    ) {
        let replacement_idx = replacement_cols.iter().position(|x| x == key_col);

        column_keys.push(key_col.to_string());
        if let Value::String(value) = value {
            //TODO!: Do we need to escape the "(" and ")" ?
            let mut value = value.replace('\'', "''");
            if let Some(replacement_idx) = replacement_idx {
                let rep_vec = replacements.get(replacement_idx).unwrap();
                value = value.replace(&rep_vec.0, &rep_vec.1);
            }
            values.push(format!("'{value}'"));
        } else if let Value::Number(value) = value {
            values.push(format!("{value}"));
        }
    }

    fn insert_to_table(
        column_keys: &mut Vec<String>,
        json_id: &i64,
        values: &mut Vec<String>,
        table: &str,
        conn: &Connection,
    ) {
        column_keys.push("json_id".to_owned());
        values.push(format!("{json_id}"));
        let stmt = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table,
            column_keys.join(", "),
            values.join(", ")
        );
        Self::db_exec(conn, &stmt);
    }

    fn load_key_value_table(
        keyvalue: &[String],
        json_id: i64,
        content: &HashMap<String, Value>,
        conn: &Connection,
    ) {
        for key in keyvalue {
            if let Some(value) = content.get(key) {
                if let Some(value) = value.as_u64() {
                    let query = format!(
                        "INSERT INTO keyvalue (key, value, json_id) VALUES ('{key}', {value}, {json_id})"
                    );
                    Self::db_exec(conn, &query);
                } else if let Some(value) = value.as_str() {
                    let query = format!(
                        "INSERT INTO keyvalue (key, value, json_id) VALUES ('{key}', '{value}', {json_id})"
                    );
                    Self::db_exec(conn, &query);
                }
            } else {
                warn!("Data missing for {key} in json {json_id}");
            }
        }
    }

    fn db_exec(conn: &Connection, query: &str) {
        let execute_query = || -> rusqlite::Result<usize> {
            // TODO: Take parameters as input of the function
            conn.execute(query, params![])
        };

        let res = execute_query();

        if let Err(code) = res {
            //TODO!: We may receive non existing columns in the table as input, so we need to handle such cases
            error!(
                "Db command execution failed, query: {query}, code: {code}"
            );
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
