//! `import`.

use std::io::Read;

use scry_core::backup::{ImportMode, ImportReport};
use serde_json::json;

use crate::client::Client;
use crate::error::AppError;
use crate::output::print_json;

/// Read a backup from stdin and import it.
pub async fn import(client: &Client, mode: ImportMode, output_json: bool) -> Result<(), AppError> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| AppError::Internal(format!("failed to read stdin: {error}")))?;

    let backup: serde_json::Value = serde_json::from_str(&input)
        .map_err(|error| AppError::Usage(format!("invalid backup JSON: {error}")))?;

    let report: ImportReport = client
        .post_json(&["import"], &[], &json!({ "backup": backup, "mode": mode }))
        .await?;

    if output_json {
        print_json(&report)?;
    } else {
        println!(
            "Imported {} project(s), {} section(s), {} task(s), {} note(s)",
            report.projects, report.sections, report.tasks, report.notes
        );
    }

    Ok(())
}
