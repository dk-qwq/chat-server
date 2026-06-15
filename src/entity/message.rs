use chrono::Utc;

use chrono::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub user_name: String,
    pub content: String,

    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
}
