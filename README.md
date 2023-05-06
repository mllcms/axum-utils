# 一个 Axum 工具库

# Examples
```rust
use std::net::SocketAddr;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use mll_axum_utils::{
    log::Log,
    middleware::{
        jwt::{JwtAuth, JwtToken},
        logger::Logger,
    },
    res::Res,
    utils::{self, echo_ip_addrs},
    validation::VJsonOrForm,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:3000".parse().unwrap();
    echo_ip_addrs(&addr);
    Log::config(|c| {
        c.file_out = true;
    });

    let app = Router::new()
        .route("/index", get(index))
        .route("/login", post(login))
        .layer(JwtAuth::<Claims>::new(vec!["/login"]))
        .layer(Logger::default());

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn login(VJsonOrForm(user): VJsonOrForm<User>) -> utils::Result<String> {
    Log::debug(&user);
    let token = Claims::new(user).encode()?;
    // some validation
    Ok(Res::success("登录成功", token))
}

async fn index(Extension(token): Extension<Claims>) -> &'static str {
    Log::debug(token);
    "身份认证成功 允许访问"
}

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
struct User {
    uid: u64,
    name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    user: User,
}

impl JwtToken for Claims {}
impl Claims {
    fn new(user: User) -> Self {
        Self {
            exp: Self::duration(),
            user,
        }
    }
}

```