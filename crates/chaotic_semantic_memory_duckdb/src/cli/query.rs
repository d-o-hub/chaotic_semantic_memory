use crate::Analytics;
use crate::error::Result;

pub async fn run(analytics: &Analytics, sql: &str, format: &str) -> Result<()> {
    let rows = analytics.query(sql)?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        if rows.is_empty() {
            println!("No results.");
        } else {
            print_table(&rows);
        }
    }

    Ok(())
}

pub fn print_table(rows: &[serde_json::Value]) {
    if rows.is_empty() {
        return;
    }

    let first = &rows[0];
    let obj = match first.as_object() {
        Some(o) => o,
        None => {
            println!("{:?}", first);
            return;
        }
    };

    let keys: Vec<_> = obj.keys().collect();

    // Simple fixed-width table printing
    for key in &keys {
        print!("{:<20} ", key);
    }
    println!();

    for _ in &keys {
        print!("{:-<20} ", "");
    }
    println!();

    for row in rows {
        if let Some(obj) = row.as_object() {
            for key in &keys {
                let val = obj.get(*key).unwrap_or(&serde_json::Value::Null);
                let val_str = match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => "NULL".to_string(),
                    _ => val.to_string(),
                };
                // Truncate if too long
                let display = if val_str.len() > 19 {
                    format!("{}...", &val_str[..16])
                } else {
                    val_str
                };
                print!("{:<20} ", display);
            }
            println!();
        }
    }
}
