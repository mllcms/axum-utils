use std::{
    sync::Arc,
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    headers::{authorization::Bearer, Authorization, HeaderMapExt},
    http::Request,
    response::{IntoResponse, Response},
};
use futures_util::future::BoxFuture;
use jsonwebtoken::{DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

use crate::res::Res;

/// # Examples
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     let addr = "127.0.0.1:3000";
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
/// async fn login(Json(user): Json<User>) -> Result<Res<String>, Res<()>> {
///     let token = Claims::new(user).encode()?;
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

#[derive(Clone)]
pub struct JwtAuthService<S, T> {
    inner: S,
    filter: Arc<Vec<&'static str>>,
    claims: Arc<T>,
}

impl<S, T> Service<Request<Body>> for JwtAuthService<S, T>
where
    T: JwtToken + Sync + Send + 'static,
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
        if !self.filter.contains(&req.uri().path()) {
            let auth = match req.headers().typed_get::<Authorization<Bearer>>() {
                Some(v) => v,
                None => {
                    return Box::pin(async {
                        Ok(Res::<()>::auth("请求未携带token").into_response())
                    })
                }
            };

            let claims = match self.claims.decode(auth.token()) {
                Ok(v) => v,
                Err(err) => return Box::pin(async { Ok(err.into_response()) }),
            };
            req.extensions_mut().insert(claims);
        }

        let future = self.inner.call(req);
        Box::pin(async move {
            let response: Response = future.await?;
            Ok(response)
        })
    }
}

pub trait JwtToken
where
    Self: Serialize + for<'a> Deserialize<'a>,
{
    /// token 编码
    fn encode(&self) -> Result<String, Res<()>> {
        let res = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            self,
            &EncodingKey::from_secret(Self::secret().as_bytes()),
        );
        match res {
            Ok(res) => Ok(res),
            Err(err) => Err(Res::error(err.to_string())),
        }
    }

    /// token 解码
    fn decode(&self, token: &str) -> Result<Self, Res<()>> {
        let res = jsonwebtoken::decode::<Self>(
            token,
            &DecodingKey::from_secret(Self::secret().as_bytes()),
            &Validation::default(),
        );
        match res {
            Ok(res) => Ok(res.claims),
            Err(err) => Err(Res::auth(err.to_string())),
        }
    }

    // token key
    fn secret() -> &'static str {
        "mykey"
    }

    // token 过期时间 默认15天
    fn duration() -> u64 {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        timestamp + 60 * 60 * 24 * 15
    }
}
