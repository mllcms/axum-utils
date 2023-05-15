use axum::extract::rejection::{BytesRejection, RawFormRejection};
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
pub struct VJson<T: Validate>(pub T);

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

        let data = des_json(Bytes::from_request(req, state).await)?;
        Ok(VJson(data))
    }
}

/// 提取 Form 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct VForm<T: Validate>(pub T);

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
        let data = des_form(req.extract::<RawForm, _>().await)?;
        Ok(VForm(data))
    }
}

/// 提取 Json 或者 Form 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct VJsonOrForm<T: Validate>(pub T);

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
            des_json(Bytes::from_request(req, state).await)?
        } else {
            des_form(req.extract::<RawForm, _>().await)?
        };

        Ok(VJsonOrForm(data))
    }
}

/// 提取 Query 类型数据 并验证数据
#[must_use]
#[derive(Debug, Clone, Copy, Default)]
pub struct VQuery<T: Validate>(pub T);

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

        validate(&data)?;
        Ok(VQuery(data))
    }
}

/// 判断 json 请求头
pub fn json_content_type(headers: &HeaderMap) -> bool {
    headers
        .typed_get::<ContentType>()
        .map(|t| t.to_string() == "application/json")
        .unwrap_or(false)
}

/// 数据验证
pub fn validate(data: impl Validate) -> Result<(), Res<Vec<String>>> {
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
    Ok(())
}

/// 返序列化 json
fn des_json<T>(data: Result<Bytes, BytesRejection>) -> Result<T, Res<Vec<String>>>
where
    T: Validate + DeserializeOwned,
{
    let bytes = data.map_err(|_| Res::validate_failed(""))?;
    let data = serde_json::from_slice::<T>(&bytes).map_err(|e| {
        Res::validate_failed(e.to_string().split(" at line").next().unwrap_or_default())
    })?;

    validate(&data)?;
    Ok(data)
}

/// 返序列化 form
fn des_form<T>(data: Result<RawForm, RawFormRejection>) -> Result<T, Res<Vec<String>>>
where
    T: Validate + DeserializeOwned,
{
    let data = match data {
        Ok(RawForm(bytes)) => serde_urlencoded::from_bytes::<T>(&bytes)
            .map_err(|err| Res::validate_failed(err.to_string()))?,
        Err(_) => return Err(Res::validate_failed("无法获取到表单数据")),
    };

    validate(&data)?;
    Ok(data)
}
