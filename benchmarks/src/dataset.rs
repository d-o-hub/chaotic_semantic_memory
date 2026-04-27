use crate::types::{QueryCase, Session};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

fn read_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(file);

    reader
        .lines()
        .map(|line| {
            let line = line?;
            Ok(serde_json::from_str::<T>(&line)?)
        })
        .collect()
}

pub fn load_sessions(path: &Path) -> Result<Vec<Session>> {
    read_jsonl(path)
}

pub fn load_queries(path: &Path) -> Result<Vec<QueryCase>> {
    read_jsonl(path)
}
