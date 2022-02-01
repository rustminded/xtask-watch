use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    first_field: String,
    second_field: String,
    third_field: String,
}

impl Config {
    pub fn new() -> Self {
        Self {
            first_field: "One".to_string(),
            second_field: "Two".to_string(),
            third_field: "Three".to_string(),
        }
    }
}
