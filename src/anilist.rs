use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::Message;

// CREATE TABLE anilist (
//     anilist_id BIGINT NOT NULL,
//     anilist_name TEXT NOT NULL,
//     site_url TEXT NOT NULL,
//     channel_id BIGINT NOT NULL,
//     added_at TIMESTAMPTZ NOT NULL,
//     added_by BIGINT NOT NULL,
//     PRIMARY KEY(anilist_id, channel_id),
//     FOREIGN KEY (channel_id) REFERENCES channels (channel_id)
//   )

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    anilist_id: u64, 
    anilist_name: String,
    site_url: String,
    channel_id: u64,
    added_by: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum User {
    NAME(String),
    URL(String),
}

pub async fn add_user (
    Json(payload): Json<User>,
) -> (StatusCode, Json<Message>) {

    let msg = match payload {
        User::NAME(name) => format!("Added user by AniList name: {}", name),
        User::URL(url) => format!("Added user by AniList URL: {}", url),
    };

    let msg = Message {
        message: format!("Added {msg}"),
    };

    (StatusCode::OK, Json(msg))
}

pub async fn remove_user (
    Json(payload): Json<User>,
) -> (StatusCode, Json<Message>) {

    (StatusCode::OK, Json())
}