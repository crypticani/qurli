use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseState {
    pub status: u16,
    pub status_text: String,
    pub duration: u64,
    pub headers: Vec<(String, String)>,
    pub body: String,
}
