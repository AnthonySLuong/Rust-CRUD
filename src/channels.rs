// CREATE TABLE channels (
//     channel_id BIGINT NOT NULL PRIMARY KEY,
//     channel_name TEXT NOT NULL,
//     guild_id BIGINT NOT NULL,
//     guild_name TEXT NOT NULL,
//     added_at TIMESTAMPTZ NOT NULL,
//     added_by BIGINT NOT NULL,
//     suppress BOOLEAN NOT NULL
//   )

use crate::{util::error_handling::internal_error, Message};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use deadpool_postgres::{GenericClient, Pool};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_postgres::error::DbError;

#[derive(Serialize, Deserialize)]
pub struct AddChannel {
    channel_id: i64,
    channel_name: String,
    guild_id: i64,
    guild_name: String,
    added_by: i64,
    suppress: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct ChannelData {
    channel_id: Option<i64>,
    channel_name: Option<String>,
    guild_id: Option<i64>,
    guild_name: Option<String>,
    suppress: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct ChannelID {
    channel_id: i64,
}

pub async fn add_channel(
    State(pool): State<Arc<Pool>>,
    Json(payload): Json<AddChannel>,
) -> Result<StatusCode, (StatusCode, Json<Message>)> {
    let pool = Arc::clone(&pool);
    let con = pool
        .get()
        .await
        .map_err(|err| internal_error(Box::new(err)))?;

    let statement = con
        .prepare("INSERT INTO channels VALUES ($1, $2, $3, $4, NOW(), $5, $6)")
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    let _result = con
        .execute(
            &statement,
            &[
                &payload.channel_id,
                &payload.channel_name,
                &payload.guild_id,
                &payload.guild_name,
                &payload.added_by,
                &payload.suppress.unwrap_or_default(),
            ],
        )
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    Ok(StatusCode::CREATED)
}

pub async fn get_channel(
    State(pool): State<Arc<Pool>>,
    Path(ChannelID { channel_id }): Path<ChannelID>,
) -> Result<(), (StatusCode, Json<Message>)> {
    let pool = Arc::clone(&pool);
    let con = pool
        .get()
        .await
        .map_err(|err| internal_error(Box::new(err)))?;

    let statement = con
        .prepare("SELECT * FROM channels WHERE channel_id = $1")
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    let result = con
        .query_opt(&statement, &[&channel_id])
        .await
        .map_err(|err| internal_error(Box::new(err)))?;

    match result {
        Some(row) => {
            for i in 0..row.len() {
                row.try_get()
            }
        }
        None => return Ok(()),
    };

    Ok(())
}

pub async fn update_channel(
    State(pool): State<Arc<Pool>>,
    Path(ChannelID { channel_id }): Path<ChannelID>,
    Json(payload): Json<UpdateChannel>,
) -> Result<StatusCode, (StatusCode, Json<Message>)> {
    let pool = Arc::clone(&pool);
    let con = pool
        .get()
        .await
        .map_err(|err| internal_error(Box::new(err)))?;

    // TODO: add more fields
    let statement = con
        .prepare("UPDATE channels SET suppress = COALESCE($1, suppress) WHERE channel_id = $2")
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    let suppress = match &payload.suppress {
        Some(value) => value.to_string(),
        None => "null".to_string(),
    };

    let _result = con
        .execute(&statement, &[&suppress, &channel_id])
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    Ok(StatusCode::CREATED)
}

pub async fn delete_channel(
    State(pool): State<Arc<Pool>>,
    Path(ChannelID { channel_id }): Path<ChannelID>,
) -> Result<StatusCode, (StatusCode, Json<Message>)> {
    let pool = Arc::clone(&pool);
    let con = pool
        .get()
        .await
        .map_err(|err| internal_error(Box::new(err)))?;

    let statement = con
        .prepare("DELETE FROM channels WHERE channel_id = $1")
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    let _result = con
        .execute(&statement, &[&channel_id])
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    Ok(StatusCode::OK)
}

// -------------------------------
// Testing
// -------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{channels, tests::pool};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{delete, get, post, put},
        Router,
    };
    use rand::{distributions::Alphanumeric, random, thread_rng, Rng};
    use serde_json::to_string;
    use tower::{Service, ServiceExt};

    async fn init() -> Router {
        let pool = pool();
        let con = pool.get().await.unwrap();
        con.simple_query(
            "CREATE TABLE IF NOT EXISTS channels (
            channel_id BIGINT NOT NULL PRIMARY KEY,
            channel_name TEXT NOT NULL,
            guild_id BIGINT NOT NULL,
            guild_name TEXT NOT NULL,
            added_at TIMESTAMPTZ NOT NULL,
            added_by BIGINT NOT NULL,
            suppress BOOLEAN NOT NULL
            )",
        )
        .await
        .unwrap();
        con.simple_query("DELETE FROM channels").await.unwrap();

        let arc_pool = Arc::new(pool);
        Router::new()
            .route("/channel", post(channels::add_channel))
            .route("/channel/:channelid", put(channels::update_channel))
            .route("/channel/:channelid", delete(channels::delete_channel))
            .with_state(arc_pool)
    }

    fn rng_add_channel() -> AddChannel {
        AddChannel {
            channel_id: random::<i64>(),
            channel_name: thread_rng()
                .sample_iter(&Alphanumeric)
                .take(10)
                .map(char::from)
                .collect(),
            guild_id: random::<i64>(),
            guild_name: thread_rng()
                .sample_iter(&Alphanumeric)
                .take(10)
                .map(char::from)
                .collect(),
            added_by: random::<i64>(),
            suppress: Some(false),
        }
    }

    #[tokio::test]
    async fn create_channel() {
        let app = init().await;
        let data = rng_add_channel();
        let json_string = to_string(&data).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/channel")
                    .header("Content-Type", "application/json")
                    .body(Body::from(json_string))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn create_channel_twice() {
        let mut app = init().await.into_service();
        let data = rng_add_channel();
        let json_string = to_string(&data).unwrap();
        let request = Request::builder()
            .method("POST")
            .uri("/channel")
            .header("Content-Type", "application/json")
            .body(Body::from(json_string.clone()))
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let request1 = Request::builder()
            .method("POST")
            .uri("/channel")
            .header("Content-Type", "application/json")
            .body(Body::from(json_string.clone()))
            .unwrap();

        let response1 = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request1)
            .await
            .unwrap();

        assert_eq!(response1.status(), StatusCode::CONFLICT);
    }
}
