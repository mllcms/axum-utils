use crate::res::Res;
use axum::async_trait;
use axum::extract::{FromRequest, FromRequestParts};
use axum::http::request::Parts;
use axum::http::Request;
use bb8::{Pool, PooledConnection};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;

pub type PgPool = AsyncDieselConnectionManager<AsyncPgConnection>;

/// 新建 pg 异步连接池
pub async fn new_pg_pool(database_url: &str) -> Pool<PgPool> {
    let config: PgPool = PgPool::new(database_url);
    Pool::builder()
        .build(config)
        .await
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

/// 获取 pg 连接
pub struct PgConn(pub PooledConnection<'static, PgPool>);

#[async_trait]
impl<S, B> FromRequest<S, B> for PgConn
where
    B: Send + 'static,
    S: Send + Sync,
{
    type Rejection = Res<()>;

    async fn from_request(req: Request<B>, _: &S) -> Result<Self, Self::Rejection> {
        let pool = req
            .extensions()
            .get::<Pool<PgPool>>()
            .expect("未设置 PgPool")
            .clone();

        let conn = pool.get_owned().await.map_err(Res::internal_error)?;

        Ok(Self(conn))
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for PgConn
where
    S: Send + Sync,
{
    type Rejection = Res<()>;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let pool = parts
            .extensions
            .get::<Pool<PgPool>>()
            .expect("未设置 PgPool")
            .clone();

        let conn = pool.get_owned().await.map_err(Res::internal_error)?;

        Ok(Self(conn))
    }
}
