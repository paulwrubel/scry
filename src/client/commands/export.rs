//! `export`.

use scry_core::backup::Backup;

use crate::client::Client;
use crate::config::ScryConfig;
use crate::error::AppError;

use super::project_by_name;

/// Export all data, or one project when `-p` is set.
pub async fn export(
    client: &Client,
    config: &ScryConfig,
    project_flag: Option<&str>,
) -> Result<(), AppError> {
    let backup: Backup = match project_flag {
        Some(_) => {
            let project = project_by_name(client, config, project_flag).await?;
            let project_id = project.id.to_string();
            client
                .get_json(&["projects", project_id.as_str(), "export"], &[])
                .await?
        }
        None => client.get_json(&["export"], &[]).await?,
    };

    let json = serde_json::to_string_pretty(&backup)
        .map_err(|error| AppError::Internal(format!("failed to serialize backup: {error}")))?;
    println!("{json}");

    Ok(())
}
