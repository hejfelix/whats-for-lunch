use serde::Serialize;

use crate::Markdown;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)] // Ephemeral not used currently
enum MattermostResponseType {
    InChannel,
    Ephemeral,
}

#[derive(Serialize)]
pub(crate) struct MattermostCommandResponse {
    text: String,
    response_type: MattermostResponseType,
}

impl MattermostCommandResponse {
    pub fn in_channel(markdown: Markdown) -> Self {
        Self {
            text: markdown.0,
            response_type: MattermostResponseType::InChannel,
        }
    }

    #[allow(dead_code)] // Ephemeral not used currently
    pub fn ephemeral(markdown: Markdown) -> Self {
        Self {
            text: markdown.0,
            response_type: MattermostResponseType::Ephemeral,
        }
    }
}