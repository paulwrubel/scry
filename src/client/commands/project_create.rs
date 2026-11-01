//! `project create`.

use scry_core::models::Project;
use serde::Serialize;

use crate::client::Client;
use crate::error::AppError;
use crate::output::print_json;

/// Input for `project create`.
pub struct CreateProject<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub template: Option<&'a str>,
}

/// Create a project.
pub async fn project_create(
    client: &Client,
    input: CreateProject<'_>,
    output_json: bool,
) -> Result<(), AppError> {
    #[derive(Serialize)]
    struct CreateProjectRequest<'a> {
        name: &'a str,
        description: Option<&'a str>,
        template: Option<&'a str>,
    }

    let project: Project = client
        .post_json(
            &["projects"],
            &[],
            &CreateProjectRequest {
                name: input.name,
                description: input.description,
                template: input.template,
            },
        )
        .await?;

    if output_json {
        print_json(&project)?;
    } else {
        println!("Created project \"{}\"", project.name);
        if input.template.is_none() {
            println!("Note: sections are optional. Add one with 'scry section add <name>'.");
        }
        println!(
            "Make it active with 'scry project use \"{}\"'.",
            project.name
        );
    }

    Ok(())
}
