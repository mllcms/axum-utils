use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Extension, Router,
};
use mll_axum_utils::middleware::interceptor;
use mll_axum_utils::{
    log::Log,
    middleware::{
        jwt::{JwtAuth, JwtToken},
        logger::Logger,
    },
    res::Res,
    utils::{self, echo_ip_addrs},
    validator::VJsonOrForm,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:3000".parse().unwrap();
    echo_ip_addrs(&addr);
    Log::config(|c| c.file_out = true);

    let app = Router::new()
        .route("/index", get(index))
        .route("/login", post(login))
        // jwt 验证
        .layer(JwtAuth::<Claims>::new(vec!["/login"]))
        // 拦截器拦截黑名单 ip 访问
        .layer(interceptor::blacklist_ip(vec!["127.0.0.1"]))
        // 访问日志记录
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

    // 数据验证
    #[validate(length(min = 3, max = 24, message = "用户名长度必须在3-24之间"))]
    name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    user: User,
}

impl JwtToken for Claims {
    const SECRET: &'static str = "new_key";
    const DURATION: u64 = 60 * 60 * 24; // token 有效期持续 1 天
}

impl Claims {
    fn new(user: User) -> Self {
        Self {
            exp: Self::expiration(),
            user,
        }
    }
}
