use std::{
    sync::Arc,
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    async_trait,
    body::Body,
    extract::FromRequest,
    headers::{authorization::Bearer, Authorization, HeaderMapExt},
    http::Request,
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use jsonwebtoken::{DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

use crate::res::Res;

/// 验证 toekn 并解析 token 携带的数据
#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct Jwt<T: JwtToken + Default>(pub T);

#[async_trait]
impl<T, S, B> FromRequest<S, B> for Jwt<T>
where
    T: JwtToken + Default,
    B: Send + 'static,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request<B>, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = auth_token::<T, B>(&req)?;
        Ok(Jwt(claims))
    }
}

fn auth_token<T: JwtToken + Default, B>(req: &Request<B>) -> Result<T, Response> {
    let auth = req
        .headers()
        .typed_get::<Authorization<Bearer>>()
        .ok_or(Res::<()>::auth("请求未携带token").into_response())?;

    let claims = T::default()
        .decode(auth.token())
        .map_err(|err| err.into_response())?;

    Ok(claims)
}

/// # Examples
/// ```no_run
/// use std::net::SocketAddr;
/// use axum::{Router,Json,Extension};
/// use axum::routing::{get, post};
/// use mll_axum_utils::middleware::jwt::{JwtAuth,JwtToken};
/// use mll_axum_utils::middleware::logger::Logger;
/// use mll_axum_utils::{utils,res::Res};
///
/// #[tokio::main]
/// async fn main() {
/// let addr = "127.0.0.1:3000";
///     let app = Router::new()
///         .route("/index", get(index))
///         .route("/login", post(login))
///         .layer(JwtAuth::<Claims>::new(vec!["/login"]))
///         .layer(Logger::default());
///
///     axum::Server::bind(&addr.parse().unwrap())
///         .serve(app.into_make_service_with_connect_info::<SocketAddr>())
///         .await
///         .unwrap();
/// }
///
/// async fn login(Json(user): Json<User>) -> utils::Result<String> {
/// let token = Claims::new(user).encode()?;
///     // some validation
///     Ok(Res::success("登录成功", token))
/// }
///
/// async fn index(Extension(token): Extension<Claims>) -> &'static str {
///     println!("{:?}", token);
///     "身份认证成功 允许访问"
/// }
///
/// #[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// struct User {
///     uid: u64,
///     name: String,
/// }
///
/// #[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// struct Claims {
///     exp: u64,
///     user: User,
/// }
///
/// impl JwtToken for Claims {}
/// impl Claims {
///     fn new(user: User) -> Self {
///         Self {
///             exp: Self::duration(),
///             user,
///         }
///     }
/// }
/// ```
#[derive(Clone)]
pub struct JwtAuth<T> {
    filter: Arc<Vec<&'static str>>,
    claims: Arc<T>,
}

impl<T> JwtAuth<T>
where
    T: Default + JwtToken,
{
    #[allow(dead_code)]
    pub fn new(filter: Vec<&'static str>) -> Self {
        Self {
            filter: Arc::new(filter),
            claims: Arc::new(T::default()),
        }
    }
}

impl<S, T> Layer<S> for JwtAuth<T>
where
    T: Default + JwtToken,
{
    type Service = JwtAuthService<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthService {
            inner,
            filter: self.filter.clone(),
            claims: self.claims.clone(),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct JwtAuthService<S, T> {
    inner: S,
    filter: Arc<Vec<&'static str>>,
    claims: Arc<T>,
}

impl<S, T> Service<Request<Body>> for JwtAuthService<S, T>
where
    T: JwtToken + Default + Sync + Send + 'static,
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let mut response = None;

        if !self.filter.contains(&req.uri().path()) {
            match auth_token::<T, _>(&req) {
                Ok(claims) => {
                    req.extensions_mut().insert(claims);
                }
                Err(err_res) => response = Some(err_res),
            }
        }

        let future = self.inner.call(req);
        Box::pin(async move {
            let response = match response {
                Some(v) => v,
                None => future.await?,
            };
            Ok(response)
        })
    }
}

pub trait JwtToken
where
    Self: Serialize + for<'a> Deserialize<'a>,
{
    /// token key
    const SECRET: &'static str = "my_key";

    /// token 持续时间 默认15天 单位 s
    const DURATION: u64 = 60 * 60 * 24 * 15;

    /// token 编码
    fn encode(&self) -> Result<String, Res<()>> {
        let res = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            self,
            &EncodingKey::from_secret(Self::SECRET.as_bytes()),
        );

        res.map_err(|err| Res::error(err.to_string()))
    }

    /// token 解码
    fn decode(&self, token: &str) -> Result<Self, Res<()>> {
        let res = jsonwebtoken::decode::<Self>(
            token,
            &DecodingKey::from_secret(Self::SECRET.as_bytes()),
            &Validation::default(),
        );
        match res {
            Ok(res) => Ok(res.claims),
            Err(err) => Err(Res::auth(err.to_string())),
        }
    }

    /// token 过期时间: 当前时间 + Self::DURATION
    fn expiration() -> u64 {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        timestamp + Self::DURATION
    }
}
