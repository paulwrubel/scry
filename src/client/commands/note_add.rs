//! `note add`.

use scry_core::models::Note;
use serde::Serialize;

use crate::client::Client;
use crate::error::AppError;
use crate::output::print_json;

/// Add a note to a task.
pub async fn note_add(
    client: &Client,
    handle: &str,
    contents: &str,
    output_json: bool,
) -> Result<(), AppError> {
    #[derive(Serialize)]
    struct NoteRequest<'a> {
        contents: &'a str,
    }

    let note: Note = client
        .post_json(
            &["tasks", handle, "notes"],
            &[],
            &NoteRequest { contents },
        )
        .await?;

    if output_json {
        print_json(&note)?;
    } else {
        println!("Added note to task {}.", handle);
    }

    Ok(())
}
