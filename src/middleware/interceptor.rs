use std::{
    sync::Arc,
    task::{Context, Poll},
};
use std::net::SocketAddr;

use axum::{body::Body, http::Request, response::Response};
use axum::extract::ConnectInfo;
use axum::response::IntoResponse;
use futures_util::future::BoxFuture;
use tower::{Layer, Service};
use crate::res::Res;

/// 前置拦截器
type Before<T> = fn(store: Arc<T>, req: &mut Request<Body>) -> Result<(), Response>;
/// 后置拦截器
type After<T> = fn(store: Arc<T>, &mut Response);

/// 拦截器
///
/// before 前置拦截器：可以修改请求体和拒绝请求
///
/// after  后置拦截器：可以修改响应体
///
/// Examples
/// ```no_run
/// use std::net::SocketAddr;
/// use std::sync::Arc;
/// use axum::body::Body;
/// use axum::extract::ConnectInfo;
/// use axum::{http::Request,response::{Response,IntoResponse}};
/// use mll_axum_utils::middleware::interceptor::Interceptor;
/// use mll_axum_utils::res::Res;
/// /// 拒绝黑名单 ip 访问
/// pub fn blacklist_ip(blacklist: Vec<&str>) -> Interceptor<Vec<&str>> {
///     fn handler(store: Arc<Vec<&str>>, req: &mut Request<Body>) -> Result<(), Response>{
///         if let Some(info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
///             if store.contains(&info.ip().to_string().as_str()) {
///                 return Err(Res::<()>::reject("").into_response());
///             }
///         }
///         Ok(())
///     }
///     Interceptor{store:Arc::new(blacklist),before:Some(handler), after:None}
/// }
/// ```
#[derive(Clone)]
pub struct Interceptor<T> {
    pub store: Arc<T>,
    pub before: Option<Before<T>>,
    pub after: Option<After<T>>,
}

impl<T> Interceptor<T> {
    pub fn new(store: T, before: Option<Before<T>>, after: Option<After<T>>) -> Self {
        Self { store: Arc::new(store), before, after }
    }
}


impl<S, T> Layer<S> for Interceptor<T> {
    type Service = InterceptorService<S, T>;

    fn layer(&self, inner: S) -> Self::Service {
        InterceptorService {
            inner,
            store: self.store.clone(),
            before: self.before,
            after: self.after,
        }
    }
}

/// 拦截器服务
#[derive(Clone)]
pub struct InterceptorService<S, T> {
    pub inner: S,
    pub store: Arc<T>,
    pub before: Option<Before<T>>,
    pub after: Option<After<T>>,
}

impl<S, T> Service<Request<Body>> for InterceptorService<S, T>
    where
        T: Sync + Send + 'static,
        S: Service<Request<Body>, Response=Response> + Send + 'static,
        S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let store = self.store.clone();
        // 执行前置拦截
        let before = self.before.and_then(|f| (f)(store.clone(), &mut req).err());
        let after = self.after;

        let future = self.inner.call(req);
        Box::pin(async move {
            // 前置拦截拒绝请求
            if let Some(err_res) = before {
                return Ok(err_res);
            }

            let mut response = future.await?;
            // 执行后置拦截
            if let Some(f) = after {
                (f)(store.clone(), &mut response);
            };
            Ok(response)
        })
    }
}

/// 拒绝黑名单 ip 访问
pub fn blacklist_ip(blacklist: Vec<&str>) -> Interceptor<Vec<&str>> {
    fn handler(store: Arc<Vec<&str>>, req: &mut Request<Body>) -> Result<(), Response> {
        if let Some(info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
            if store.contains(&info.ip().to_string().as_str()) {
                return Err(Res::<()>::reject("").into_response());
            }
        }
        Ok(())
    }
    Interceptor { store: Arc::new(blacklist), before: Some(handler), after: None }
}