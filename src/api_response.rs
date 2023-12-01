use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct ApiResponse {
    pub status: u16,
    pub msg: String,
    pub description: String,
    pub data: Vec<Value>,
}
