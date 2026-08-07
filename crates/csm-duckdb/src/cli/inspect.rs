use crate::Analytics;
use crate::error::Result;
use std::io::{self, Write};

pub async fn run(analytics: &mut Analytics) -> Result<()> {
    println!("CSM SQL REPL (DuckDB)");
    println!("Type '.exit' or '.quit' to leave.");

    loop {
        print!("sql> ");
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            println!();
            break;
        }
        let sql = input.trim();

        if sql.is_empty() {
            continue;
        }

        if sql == ".exit" || sql == ".quit" {
            break;
        }

        match analytics.query(sql) {
            Ok(rows) => {
                if rows.is_empty() {
                    println!("No results.");
                } else {
                    crate::cli::query::print_table(&rows);
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
    }

    Ok(())
}
