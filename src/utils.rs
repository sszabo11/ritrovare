use rusqlite::types::Value;
use url::Url;

use crate::browser::Tab;

pub fn parse_val_str(val: Value) -> String {
    match val {
        Value::Null => String::new(),
        Value::Integer(n) => n.to_string(),
        Value::Real(f) => f.to_string(),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => format!("<blob {} bytes>", b.len()),
    }
}

pub fn filter_tabs(tabs: Vec<Tab>) -> Vec<Tab> {
    let filtered: Vec<Tab> = tabs
        .into_iter()
        .filter(|t| !t.url.is_empty() && !t.title.is_empty() && is_url(&t.url))
        .collect();

    filtered
}

pub fn is_url(url: &str) -> bool {
    let parsed = Url::parse(url).expect("Failed to parse url");

    let is_domain = parsed.domain().is_some();
    if !is_domain {
        log::info!("Invalid domain")
    }
    is_domain
}
