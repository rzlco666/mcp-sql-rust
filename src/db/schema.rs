use serde::{Deserialize, Serialize};

use crate::db::{EngineKind, EnginePool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ObjectType {
    Schema,
    Table,
    Column,
    Index,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaObject {
    pub object_type: ObjectType,
    pub schema: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<ColumnInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexes: Option<Vec<IndexInfo>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub objects: Vec<SchemaObject>,
    pub meta: SearchMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchMeta {
    pub n: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

pub async fn search_objects(
    pool: &EnginePool,
    object_type: ObjectType,
    keyword: Option<&str>,
    schema: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<SearchResult, sqlx::Error> {
    let mut objects = match object_type {
        ObjectType::Schema => list_schemas(pool, keyword).await?,
        ObjectType::Table => list_tables(pool, schema, keyword).await?,
        ObjectType::Column => list_columns(pool, schema, keyword).await?,
        ObjectType::Index => list_indexes(pool, schema, None, keyword).await?,
    };

    let total = objects.len();
    let slice = objects
        .drain(offset..total.min(offset + limit))
        .collect::<Vec<_>>();

    Ok(SearchResult {
        meta: SearchMeta {
            n: slice.len(),
            offset,
            limit,
            has_more: offset + slice.len() < total,
        },
        objects: slice,
    })
}

pub async fn list_schemas(
    pool: &EnginePool,
    keyword: Option<&str>,
) -> Result<Vec<SchemaObject>, sqlx::Error> {
    match pool.engine() {
        EngineKind::Postgres => {
            let pool = pool.postgres().expect("postgres");
            let rows = sqlx::query(
                "SELECT schema_name FROM information_schema.schemata \
                 WHERE schema_name NOT IN ('pg_catalog', 'information_schema', 'pg_toast') \
                 ORDER BY schema_name",
            )
            .fetch_all(pool)
            .await?;
            Ok(filter_objects(
                rows.iter()
                    .filter_map(|r| {
                        let name: String = r.try_get(0).ok()?;
                        Some(SchemaObject {
                            object_type: ObjectType::Schema,
                            schema: None,
                            name,
                            table: None,
                            data_type: None,
                            nullable: None,
                            columns: None,
                            indexes: None,
                        })
                    })
                    .collect(),
                keyword,
            ))
        }
        EngineKind::Mysql => {
            let pool = pool.mysql().expect("mysql");
            let rows = sqlx::query("SHOW DATABASES").fetch_all(pool).await?;
            Ok(filter_objects(
                rows.iter()
                    .filter_map(|r| {
                        let name: String = r.try_get(0).ok()?;
                        Some(SchemaObject {
                            object_type: ObjectType::Schema,
                            schema: None,
                            name,
                            table: None,
                            data_type: None,
                            nullable: None,
                            columns: None,
                            indexes: None,
                        })
                    })
                    .collect(),
                keyword,
            ))
        }
    }
}

pub async fn list_tables(
    pool: &EnginePool,
    schema: Option<&str>,
    keyword: Option<&str>,
) -> Result<Vec<SchemaObject>, sqlx::Error> {
    match pool.engine() {
        EngineKind::Postgres => {
            let pool = pool.postgres().expect("postgres");
            let schema = schema.unwrap_or("public");
            let rows = sqlx::query(
                "SELECT table_name FROM information_schema.tables \
                 WHERE table_schema = $1 AND table_type = 'BASE TABLE' \
                 ORDER BY table_name",
            )
            .bind(schema)
            .fetch_all(pool)
            .await?;
            Ok(filter_objects(
                rows.iter()
                    .filter_map(|r| {
                        let name: String = r.try_get(0).ok()?;
                        Some(SchemaObject {
                            object_type: ObjectType::Table,
                            schema: Some(schema.into()),
                            name,
                            table: None,
                            data_type: None,
                            nullable: None,
                            columns: None,
                            indexes: None,
                        })
                    })
                    .collect(),
                keyword,
            ))
        }
        EngineKind::Mysql => {
            let pool = pool.mysql().expect("mysql");
            let schema = schema.unwrap_or("");

            let rows = if schema.is_empty() {
                sqlx::query(
                    "SELECT table_schema, table_name FROM information_schema.tables \
                     WHERE table_type = 'BASE TABLE' ORDER BY table_schema, table_name",
                )
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query(
                    "SELECT table_schema, table_name FROM information_schema.tables \
                     WHERE table_schema = ? AND table_type = 'BASE TABLE' ORDER BY table_name",
                )
                .bind(schema)
                .fetch_all(pool)
                .await?
            };

            Ok(filter_objects(
                rows.iter()
                    .filter_map(|r| {
                        let sch: String = r.try_get(0).ok()?;
                        let name: String = r.try_get(1).ok()?;
                        Some(SchemaObject {
                            object_type: ObjectType::Table,
                            schema: Some(sch),
                            name,
                            table: None,
                            data_type: None,
                            nullable: None,
                            columns: None,
                            indexes: None,
                        })
                    })
                    .collect(),
                keyword,
            ))
        }
    }
}

async fn list_columns(
    pool: &EnginePool,
    schema: Option<&str>,
    keyword: Option<&str>,
) -> Result<Vec<SchemaObject>, sqlx::Error> {
    match pool.engine() {
        EngineKind::Postgres => {
            let pool = pool.postgres().expect("postgres");
            let schema = schema.unwrap_or("public");
            let rows = sqlx::query(
                "SELECT table_name, column_name, data_type, is_nullable \
                 FROM information_schema.columns \
                 WHERE table_schema = $1 ORDER BY table_name, ordinal_position",
            )
            .bind(schema)
            .fetch_all(pool)
            .await?;
            Ok(filter_objects(
                rows.iter()
                    .filter_map(|r| {
                        let table: String = r.try_get(0).ok()?;
                        let name: String = r.try_get(1).ok()?;
                        let data_type: String = r.try_get(2).ok()?;
                        let nullable: String = r.try_get(3).ok()?;
                        Some(SchemaObject {
                            object_type: ObjectType::Column,
                            schema: Some(schema.into()),
                            name,
                            table: Some(table),
                            data_type: Some(data_type),
                            nullable: Some(nullable == "YES"),
                            columns: None,
                            indexes: None,
                        })
                    })
                    .collect(),
                keyword,
            ))
        }
        EngineKind::Mysql => {
            let pool = pool.mysql().expect("mysql");
            let rows = if let Some(schema) = schema {
                sqlx::query(
                    "SELECT table_schema, table_name, column_name, data_type, is_nullable \
                     FROM information_schema.columns WHERE table_schema = ? \
                     ORDER BY table_name, ordinal_position",
                )
                .bind(schema)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query(
                    "SELECT table_schema, table_name, column_name, data_type, is_nullable \
                     FROM information_schema.columns \
                     ORDER BY table_schema, table_name, ordinal_position",
                )
                .fetch_all(pool)
                .await?
            };
            Ok(filter_objects(
                rows.iter()
                    .filter_map(|r| {
                        let sch: String = r.try_get(0).ok()?;
                        let table: String = r.try_get(1).ok()?;
                        let name: String = r.try_get(2).ok()?;
                        let data_type: String = r.try_get(3).ok()?;
                        let nullable: String = r.try_get(4).ok()?;
                        Some(SchemaObject {
                            object_type: ObjectType::Column,
                            schema: Some(sch),
                            name,
                            table: Some(table),
                            data_type: Some(data_type),
                            nullable: Some(nullable == "YES"),
                            columns: None,
                            indexes: None,
                        })
                    })
                    .collect(),
                keyword,
            ))
        }
    }
}

pub async fn describe_table(
    pool: &EnginePool,
    schema: Option<&str>,
    table: &str,
) -> Result<SchemaObject, sqlx::Error> {
    let engine = pool.engine();
    let schema = match engine {
        EngineKind::Postgres => schema.unwrap_or("public").to_string(),
        EngineKind::Mysql => schema.map(str::to_string).unwrap_or_default(),
    };

    let columns = match engine {
        EngineKind::Postgres => {
            let pool = pool.postgres().expect("postgres");
            let rows = sqlx::query(
                "SELECT column_name, data_type, is_nullable \
                 FROM information_schema.columns \
                 WHERE table_schema = $1 AND table_name = $2 \
                 ORDER BY ordinal_position",
            )
            .bind(&schema)
            .bind(table)
            .fetch_all(pool)
            .await?;
            rows.iter()
                .filter_map(|r| {
                    Some(ColumnInfo {
                        name: r.try_get(0).ok()?,
                        data_type: r.try_get(1).ok()?,
                        nullable: r.try_get::<String, _>(2).ok()? == "YES",
                    })
                })
                .collect()
        }
        EngineKind::Mysql => {
            let pool = pool.mysql().expect("mysql");
            let rows = if schema.is_empty() {
                sqlx::query(
                    "SELECT column_name, data_type, is_nullable \
                     FROM information_schema.columns WHERE table_name = ? \
                     ORDER BY ordinal_position",
                )
                .bind(table)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query(
                    "SELECT column_name, data_type, is_nullable \
                     FROM information_schema.columns \
                     WHERE table_schema = ? AND table_name = ? ORDER BY ordinal_position",
                )
                .bind(&schema)
                .bind(table)
                .fetch_all(pool)
                .await?
            };
            rows.iter()
                .filter_map(|r| {
                    Some(ColumnInfo {
                        name: r.try_get(0).ok()?,
                        data_type: r.try_get(1).ok()?,
                        nullable: r.try_get::<String, _>(2).ok()? == "YES",
                    })
                })
                .collect()
        }
    };

    let indexes = list_indexes(pool, Some(&schema), Some(table), None)
        .await?
        .into_iter()
        .map(|o| IndexInfo {
            name: o.name,
            columns: o
                .indexes
                .and_then(|i| i.first().map(|x| x.columns.clone()))
                .unwrap_or_default(),
            unique: false,
        })
        .collect();

    Ok(SchemaObject {
        object_type: ObjectType::Table,
        schema: if schema.is_empty() {
            None
        } else {
            Some(schema)
        },
        name: table.to_string(),
        table: None,
        data_type: None,
        nullable: None,
        columns: Some(columns),
        indexes: Some(indexes),
    })
}

pub async fn list_indexes(
    pool: &EnginePool,
    schema: Option<&str>,
    table: Option<&str>,
    keyword: Option<&str>,
) -> Result<Vec<SchemaObject>, sqlx::Error> {
    match pool.engine() {
        EngineKind::Postgres => {
            let pool = pool.postgres().expect("postgres");
            let schema = schema.unwrap_or("public");
            let rows = if let Some(table) = table {
                sqlx::query(
                    "SELECT indexname, indexdef FROM pg_indexes \
                     WHERE schemaname = $1 AND tablename = $2 ORDER BY indexname",
                )
                .bind(schema)
                .bind(table)
                .fetch_all(pool)
                .await?
            } else {
                sqlx::query(
                    "SELECT indexname, indexdef FROM pg_indexes \
                     WHERE schemaname = $1 ORDER BY indexname",
                )
                .bind(schema)
                .fetch_all(pool)
                .await?
            };
            Ok(filter_objects(
                rows.iter()
                    .filter_map(|r| {
                        let name: String = r.try_get(0).ok()?;
                        let def: String = r.try_get(1).ok()?;
                        Some(SchemaObject {
                            object_type: ObjectType::Index,
                            schema: Some(schema.into()),
                            name,
                            table: table.map(str::to_string),
                            data_type: Some(def),
                            nullable: None,
                            columns: None,
                            indexes: None,
                        })
                    })
                    .collect(),
                keyword,
            ))
        }
        EngineKind::Mysql => {
            let pool = pool.mysql().expect("mysql");
            let rows = match (schema, table) {
                (Some(schema), Some(table)) => {
                    sqlx::query(
                        "SELECT index_name, column_name, non_unique \
                         FROM information_schema.statistics \
                         WHERE table_schema = ? AND table_name = ? \
                         ORDER BY index_name, seq_in_index",
                    )
                    .bind(schema)
                    .bind(table)
                    .fetch_all(pool)
                    .await?
                }
                (Some(schema), None) => {
                    sqlx::query(
                        "SELECT index_name, column_name, non_unique \
                         FROM information_schema.statistics \
                         WHERE table_schema = ? ORDER BY index_name, seq_in_index",
                    )
                    .bind(schema)
                    .fetch_all(pool)
                    .await?
                }
                _ => {
                    sqlx::query(
                        "SELECT table_schema, index_name, column_name, non_unique \
                         FROM information_schema.statistics \
                         ORDER BY table_schema, index_name, seq_in_index",
                    )
                    .fetch_all(pool)
                    .await?
                }
            };

            // Group by index name
            use std::collections::BTreeMap;
            let mut map: BTreeMap<String, (Vec<String>, bool, Option<String>)> =
                BTreeMap::new();
            for row in &rows {
                if schema.is_none() && table.is_none() {
                    let sch: String = row.try_get(0).ok().unwrap_or_default();
                    let name: String = row.try_get(1).ok().unwrap_or_default();
                    let col: String = row.try_get(2).ok().unwrap_or_default();
                    let non_unique: i32 = row.try_get(3).ok().unwrap_or(1);
                    let key = format!("{sch}.{name}");
                    let entry = map.entry(key).or_insert((vec![], non_unique == 0, Some(sch)));
                    entry.0.push(col);
                } else {
                    let name: String = row.try_get(0).ok().unwrap_or_default();
                    let col: String = row.try_get(1).ok().unwrap_or_default();
                    let non_unique: i32 = row.try_get(2).ok().unwrap_or(1);
                    let entry = map
                        .entry(name.clone())
                        .or_insert((vec![], non_unique == 0, schema.map(str::to_string)));
                    entry.0.push(col);
                }
            }

            let objects = map
                .into_iter()
                .map(|(name, (columns, unique, sch))| SchemaObject {
                    object_type: ObjectType::Index,
                    schema: sch,
                    name,
                    table: table.map(str::to_string),
                    data_type: None,
                    nullable: None,
                    columns: None,
                    indexes: Some(vec![IndexInfo {
                        name: "primary".into(),
                        columns,
                        unique,
                    }]),
                })
                .collect();
            Ok(filter_objects(objects, keyword))
        }
    }
}

fn filter_objects(mut objects: Vec<SchemaObject>, keyword: Option<&str>) -> Vec<SchemaObject> {
    if let Some(kw) = keyword {
        let kw = kw.to_lowercase();
        objects.retain(|o| {
            o.name.to_lowercase().contains(&kw)
                || o.table
                    .as_ref()
                    .is_some_and(|t| t.to_lowercase().contains(&kw))
                || o.schema
                    .as_ref()
                    .is_some_and(|s| s.to_lowercase().contains(&kw))
        });
    }
    objects
}

use sqlx::Row;
