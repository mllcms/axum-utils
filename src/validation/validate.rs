use axum::{
    async_trait,
    body::HttpBody,
    extract::{FromRequest, RawForm},
    headers::{ContentType, HeaderMapExt},
    http::{HeaderMap, Request},
    BoxError, RequestExt,
};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::res::Res;

/// 提取 Json 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct VJson<T>(pub T);

#[async_trait]
impl<T, S, B> FromRequest<S, B> for VJson<T>
where
    T: DeserializeOwned + Validate,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = Res<Vec<String>>;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        if !json_content_type(req.headers()) {
            return Err(Res::validate_failed("请求头必须为: application/json"));
        }

        let body = Bytes::from_request(req, state)
            .await
            .map_err(|_| Res::validate_failed(""))?;

        let data = serde_json::from_slice::<T>(&body).map_err(|e| {
            Res::validate_failed(e.to_string().split(" at line").next().unwrap_or_default())
        })?;

        if let Err(err) = data.validate() {
            let mut err_data = Vec::new();
            for (k, v) in err.field_errors() {
                for item in v {
                    let msg = item.message.as_ref().unwrap_or(&item.code);
                    err_data.push(format!("{k:}: validate failed tips: {}", msg));
                }
            }
            return Err(Res::validate_failed_data(err_data));
        }

        Ok(VJson(data))
    }
}

pub fn json_content_type(headers: &HeaderMap) -> bool {
    headers
        .typed_get::<ContentType>()
        .map(|t| t.to_string() == "application/json")
        .unwrap_or(false)
}

/// 提取 Form 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct VForm<T>(pub T);

#[async_trait]
impl<T, S, B> FromRequest<S, B> for VForm<T>
where
    T: DeserializeOwned + Validate,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = Res<Vec<String>>;

    async fn from_request(req: Request<B>, _state: &S) -> Result<Self, Self::Rejection> {
        let data = match req.extract().await {
            Ok(RawForm(bytes)) => serde_urlencoded::from_bytes::<T>(&bytes)
                .map_err(|err| Res::validate_failed(err.to_string()))?,
            Err(_) => return Err(Res::validate_failed("无法获取到表单数据")),
        };

        if let Err(err) = data.validate() {
            let mut err_data = Vec::new();
            for (k, v) in err.field_errors() {
                for item in v {
                    let msg = item.message.as_ref().unwrap_or(&item.code);
                    err_data.push(format!("{k:}: validate failed tips: {}", msg));
                }
            }
            return Err(Res::validate_failed_data(err_data));
        }

        Ok(VForm(data))
    }
}

/// 提取 Json 或者 Form 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct VJsonOrForm<T>(pub T);

#[async_trait]
impl<T, S, B> FromRequest<S, B> for VJsonOrForm<T>
where
    T: DeserializeOwned + Validate,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = Res<Vec<String>>;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let data = if json_content_type(req.headers()) {
            let body = Bytes::from_request(req, state)
                .await
                .map_err(|_| Res::validate_failed(""))?;

            serde_json::from_slice::<T>(&body).map_err(|e| {
                Res::validate_failed(e.to_string().split(" at line").next().unwrap_or_default())
            })?
        } else {
            match req.extract().await {
                Ok(RawForm(bytes)) => serde_urlencoded::from_bytes::<T>(&bytes)
                    .map_err(|err| Res::validate_failed(err.to_string()))?,
                Err(_) => return Err(Res::validate_failed("无法获取到表单数据")),
            }
        };

        if let Err(err) = data.validate() {
            let mut err_data = Vec::new();
            for (k, v) in err.field_errors() {
                for item in v {
                    let msg = item.message.as_ref().unwrap_or(&item.code);
                    err_data.push(format!("{k:}: validate failed tips: {}", msg));
                }
            }
            return Err(Res::validate_failed_data(err_data));
        }

        Ok(VJsonOrForm(data))
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct VQuery<T>(pub T);
#[async_trait]
impl<T, S, B> FromRequest<S, B> for VQuery<T>
where
    T: DeserializeOwned + Validate,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = Res<Vec<String>>;

    async fn from_request(req: Request<B>, _state: &S) -> Result<Self, Self::Rejection> {
        let data = serde_urlencoded::from_str::<T>(req.uri().query().unwrap_or_default())
            .map_err(|err| Res::validate_failed(err.to_string()))?;

        if let Err(err) = data.validate() {
            let mut err_data = Vec::new();
            for (k, v) in err.field_errors() {
                for item in v {
                    let msg = item.message.as_ref().unwrap_or(&item.code);
                    err_data.push(format!("{k:}: validate failed tips: {}", msg));
                }
            }
            return Err(Res::validate_failed_data(err_data));
        }

        Ok(VQuery(data))
    }
}
