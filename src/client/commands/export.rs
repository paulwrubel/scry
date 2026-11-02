//! `export`.

use super::project_by_name;
use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;
use scry_core::snapshot::SnapshotFile;

/// Export all data, or one project when `-p` is set.
pub async fn export(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
) -> Result<(), AppError> {
    let file: SnapshotFile = match project_flag {
        Some(_) => {
            let project = project_by_name(client, config, project_flag).await?;
            let project_id = project.id.to_string();
            client
                .get_json(&["projects", project_id.as_str(), "export"], &[])
                .await?
        }
        None => client.get_json(&["export"], &[]).await?,
    };

    let json = serde_json::to_string_pretty(&file)
        .map_err(|error| AppError::Internal(format!("failed to serialize snapshot: {error}")))?;
    println!("{json}");

    Ok(())
}
