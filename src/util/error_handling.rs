use crate::Message;
use axum::{http::StatusCode, Json};
use std::error::Error;
use tokio_postgres::error::DbError;

pub fn internal_error(err: Box<dyn Error>) -> (StatusCode, Json<Message>) {
    if let Ok(db_error) = err.downcast::<DbError>() {
        let msg = Message {
            message: db_error.message().to_string(),
            sqlstate: Some(db_error.code().code().to_string()),
        };

        (StatusCode::CONFLICT, Json(msg))
    } else {
        let msg = Message {
            message: "INTERNAL SERVER ERROR".to_string(),
            ..Default::default()
        };

        (StatusCode::INTERNAL_SERVER_ERROR, Json(msg))
    }
}
