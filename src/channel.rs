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
pub struct Create {
    channel_id: i64,
    channel_name: String,
    guild_id: i64,
    guild_name: String,
    added_by: i64,
    suppress: Option<bool>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Data {
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guild_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guild_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suppress: Option<bool>,
}

pub async fn add(
    State(pool): State<Arc<Pool>>,
    Json(payload): Json<Create>,
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

pub async fn get(
    State(pool): State<Arc<Pool>>,
    Path(channel_id): Path<i64>,
) -> Result<Json<Data>, (StatusCode, Json<Message>)> {
    let pool = Arc::clone(&pool);
    let con = pool
        .get()
        .await
        .map_err(|err| internal_error(Box::new(err)))?;

    let statement = con
        .prepare("SELECT channel_name, guild_id, guild_name, suppress FROM channels WHERE channel_id = $1")
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    let result = con
        .query_one(&statement, &[&channel_id])
        .await
        .map_err(|_| {
            let msg = Message {
                message: format!("Could not find {channel_id}"),
                ..Default::default()
            };
            
            (StatusCode::NOT_FOUND, Json(msg))
        })?;

    let data = Data {
        channel_name: result.get("channel_name"),
        guild_id: result.get("guild_id"),
        guild_name: result.get("guild_name"),
        suppress: result.get("suppress"),
        ..Default::default()
    };

    Ok(Json(data))
}

pub async fn update(
    State(pool): State<Arc<Pool>>,
    Path(channel_id): Path<i64>,
    Json(payload): Json<Data>,
) -> Result<StatusCode, (StatusCode, Json<Message>)> {
    let pool = Arc::clone(&pool);
    let con = pool
        .get()
        .await
        .map_err(|err| internal_error(Box::new(err)))?;

    // TODO: add more fields
    let statement = con
        .prepare("UPDATE channels SET suppress = CASE WHEN $1::BOOLEAN IS NOT NULL THEN $1 ELSE suppress END WHERE channel_id = $2")
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    let _result = con
        .execute(&statement, &[&payload.suppress, &channel_id])
        .await
        .map_err(|err| {
            let db_error = DbError::clone(err.as_db_error().unwrap());
            internal_error(Box::new(db_error))
        })?;

    Ok(StatusCode::OK)
}

pub async fn delete(
    State(pool): State<Arc<Pool>>,
    Path(channel_id): Path<i64>,
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

// ------------------------------------------------
// Testing
// ------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{channel, tests::pool};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{delete, get, post, put},
        Router,
    };
    use http_body_util::BodyExt;
    use rand::{distributions::Alphanumeric, random, thread_rng, Rng};
    use serde_json::{json, to_string, Value};
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
            .route("/channel", post(channel::add))
            .route("/channel/:channelid", get(channel::get))
            .route("/channel/:channelid", put(channel::update))
            .route("/channel/:channelid", delete(channel::delete))
            .with_state(arc_pool)
    }

    fn rng_add_channel() -> Create {
        Create {
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
    async fn create_test() {
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
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn create_twice_test() {
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
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());

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

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(!body.is_empty());
    }

    #[tokio::test]
    async fn get_test() {
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
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());

        let request = Request::builder()
            .method("GET")
            .uri(format!("/channel/{}", data.channel_id))
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            json!({"channel_name": data.channel_name, "guild_id": data.guild_id, "guild_name": data.guild_name, "suppress": false})
        );
    }

    #[tokio::test]
    async fn get_invalid() {
        let app = init().await;
        let data = rng_add_channel();
        let json_string = to_string(&data).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/channel/{}", data.channel_id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(json_string))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            json!({"message": format!("Could not find {}", data.channel_id)})
        );
    }

    #[tokio::test]
    async fn update_test() {
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
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());
        
        let request = Request::builder()
            .method("PUT")
            .uri(format!("/channel/{}", data.channel_id))
            .header("Content-Type", "application/json")
            .body(Body::from("{\"suppress\": true}"))
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());

        let request = Request::builder()
            .method("GET")
            .uri(format!("/channel/{}", data.channel_id))
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            json!({"channel_name": data.channel_name, "guild_id": data.guild_id, "guild_name": data.guild_name, "suppress": true})
        );
    }
}
