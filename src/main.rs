mod channels;
mod util;
// mod anilist;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod, Runtime};
use serde::Serialize;
use std::{env, sync::Arc};
use tokio_postgres::NoTls;

#[derive(Serialize, Default)]
struct Message<T> {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Box<T>,
}

#[tokio::main]
async fn main() {
    let host = env::var("HOST").expect("");
    let db_name = env::var("DBNAME").expect("");
    let user = env::var("USER").expect("");
    let password = env::var("PASSWORD").expect("");

    let mut config = Config::new();
    config.host = Some(host);
    config.dbname = Some(db_name);
    config.user = Some(user);
    config.password = Some(password);
    config.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    let pool = config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("Couldn't create conneciton pool");

    let arc_pool = Arc::new(pool);

    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/channel", post(channels::add_channel))
        .route("/channel/:channelid", get(channels::get_channel))
        .route("/channel/:channelid", put(channels::update_channel))
        .route("/channel/:channelid", delete(channels::delete_channel))
        .with_state(arc_pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80")
        .await
        .expect("Couldn't bind tcplistener");

    axum::serve(listener, app)
        .await
        .expect("Couldn't serve service");
}

#[cfg(test)]
mod tests {
    use deadpool_postgres::{Pool, PoolConfig, Runtime};
    use tokio_postgres::NoTls;

    pub fn pool() -> Pool {
        // Docker Postgres Image
        // Database Name => anisocial
        // Default username, password => postgres
        let pool_config = PoolConfig::new(1);
        let mut config = deadpool_postgres::Config::new();
        config.host = Some("localhost".to_string());
        config.dbname = Some("anisocial".to_string());
        config.user = Some("postgres".to_string());
        config.password = Some("postgres".to_string());
        config.pool = Some(pool_config);

        config.create_pool(Some(Runtime::Tokio1), NoTls).unwrap()
    }
}
