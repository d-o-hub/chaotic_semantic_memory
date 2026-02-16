use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("sqlite error: {0}")]
    Sqlite(String),
    #[error("client error: {0}")]
    Message(String),
}

#[derive(Clone)]
pub struct Client {
    db_path: Arc<PathBuf>,
    _url: String,
    _token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Row(pub Vec<Value>);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Value {
    Text(String),
    Blob(Vec<u8>),
}

impl Row {
    pub fn get<T: FromValue>(&self, idx: usize) -> Result<T, Error> {
        self.0
            .get(idx)
            .ok_or_else(|| Error::Message("column out of bounds".to_string()))
            .and_then(T::from_value)
    }
}

pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self, Error>;
}

impl FromValue for String {
    fn from_value(value: &Value) -> Result<Self, Error> {
        match value {
            Value::Text(v) => Ok(v.clone()),
            _ => Err(Error::Message("expected text".to_string())),
        }
    }
}

impl FromValue for Vec<u8> {
    fn from_value(value: &Value) -> Result<Self, Error> {
        match value {
            Value::Blob(v) => Ok(v.clone()),
            _ => Err(Error::Message("expected blob".to_string())),
        }
    }
}

pub enum Params {
    Empty,
    Pair((String, Vec<u8>)),
}

impl From<()> for Params {
    fn from(_: ()) -> Self {
        Self::Empty
    }
}

impl From<(String, Vec<u8>)> for Params {
    fn from(value: (String, Vec<u8>)) -> Self {
        Self::Pair(value)
    }
}

impl Client {
    pub fn new(url: String, token: String) -> Result<Self, Error> {
        let db_path = parse_path_from_url(&url);
        let client = Self {
            db_path: Arc::new(db_path),
            _url: url,
            _token: token,
        };
        client.with_conn(|conn| {
            conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
                .map_err(|e| Error::Sqlite(e.to_string()))
        })?;
        Ok(client)
    }

    pub async fn execute<P>(&self, sql: &str, params: P) -> Result<QueryResult, Error>
    where
        P: Into<Params> + Send + 'static,
    {
        let db_path = self.db_path.clone();
        let sql_text = sql.to_string();
        let params = params.into();
        tokio::task::spawn_blocking(move || {
            let conn = Connection::open(&*db_path).map_err(|e| Error::Sqlite(e.to_string()))?;
            execute_sql(&conn, &sql_text, params)
        })
        .await
        .map_err(|e| Error::Message(format!("join error: {e}")))?
    }

    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T, Error>) -> Result<T, Error> {
        let conn = Connection::open(&*self.db_path).map_err(|e| Error::Sqlite(e.to_string()))?;
        f(&conn)
    }
}

fn execute_sql(conn: &Connection, sql: &str, params: Params) -> Result<QueryResult, Error> {
    if sql.starts_with("INSERT INTO concepts") {
        let (name, payload) = match params {
            Params::Pair(pair) => pair,
            _ => return Err(Error::Message("missing params".to_string())),
        };
        conn.execute(
            "INSERT INTO concepts(name, payload) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET payload = excluded.payload",
            params![name, payload],
        )
        .map_err(|e| Error::Sqlite(e.to_string()))?;
        return Ok(QueryResult { rows: vec![] });
    }

    if sql.starts_with("SELECT name,payload FROM concepts") {
        let mut stmt = conn
            .prepare("SELECT name, payload FROM concepts")
            .map_err(|e| Error::Sqlite(e.to_string()))?;
        let rows_iter = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let payload: Vec<u8> = row.get(1)?;
                Ok(Row(vec![Value::Text(name), Value::Blob(payload)]))
            })
            .map_err(|e| Error::Sqlite(e.to_string()))?;

        let mut rows = Vec::new();
        for item in rows_iter {
            rows.push(item.map_err(|e| Error::Sqlite(e.to_string()))?);
        }
        return Ok(QueryResult { rows });
    }

    conn.execute_batch(sql)
        .map_err(|e| Error::Sqlite(e.to_string()))?;
    Ok(QueryResult { rows: vec![] })
}

fn parse_path_from_url(url: &str) -> PathBuf {
    if let Some(path) = url.strip_prefix("file:") {
        return PathBuf::from(path);
    }
    if let Some(path) = url.strip_prefix("libsql://") {
        let sanitized = path.replace(['/', ':', '?', '&', '='], "_");
        return PathBuf::from(format!("{sanitized}.db"));
    }
    PathBuf::from(url)
}
