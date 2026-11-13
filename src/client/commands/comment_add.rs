//! `comment add`.

use scry_core::models::Comment;
use serde::Serialize;

use crate::client::Client;
use crate::error::AppError;
use crate::output::print_json;

/// Add a comment to a task.
pub async fn comment_add(
    client: &Client,
    handle: &str,
    contents: &str,
    output_json: bool,
) -> Result<(), AppError> {
    #[derive(Serialize)]
    struct CommentRequest<'a> {
        contents: &'a str,
    }

    let comment: Comment = client
        .post_json(
            &["tasks", handle, "comments"],
            &[],
            &CommentRequest { contents },
        )
        .await?;

    if output_json {
        print_json(&comment)?;
    } else {
        println!("Added comment to task {}.", handle);
    }

    Ok(())
}
