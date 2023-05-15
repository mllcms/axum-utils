use std::{fmt::Display, format};

use axum::{
    body::{boxed, Full},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Res<T> {
    code: u16,
    msg: String,
    data: Option<T>,
}

impl<T> IntoResponse for Res<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let body = Full::from(serde_json::to_vec(&self).unwrap());

        Response::builder()
            .status(self.code)
            .header("Content-type", "application/json")
            .body(boxed(body))
            .unwrap()
    }
}

impl<T> Res<T>
where
    T: Serialize,
{
    #[allow(dead_code)]
    pub fn new<C, M>(code: C, msg: M) -> Self
    where
        C: Into<u16>,
        M: Display,
    {
        Self {
            code: code.into(),
            msg: format!("{msg}"),
            data: None,
        }
    }

    #[allow(dead_code)]
    pub fn new_data<C, M>(code: C, msg: M, data: T) -> Self
    where
        C: Into<u16>,
        M: Display,
    {
        Self {
            code: code.into(),
            msg: format!("{msg}"),
            data: Some(data),
        }
    }

    /// 201 创建资源成功
    #[allow(dead_code)]
    pub fn created<M>(msg: M) -> Self
    where
        M: Display,
    {
        Self {
            code: StatusCode::CREATED.into(),
            msg: format!("{msg}"),
            data: None,
        }
    }

    ///  200 成功 响应数据
    #[allow(dead_code)]
    pub fn ok(data: T) -> Self {
        Self {
            code: StatusCode::OK.as_u16(),
            msg: "ok".into(),
            data: Some(data),
        }
    }

    ///  400 失败 响应消息
    #[allow(dead_code)]
    pub fn error<M>(msg: M) -> Self
    where
        M: Display,
    {
        Self {
            code: StatusCode::BAD_REQUEST.as_u16(),
            msg: format!("{msg}"),
            data: None,
        }
    }

    ///  401 身份认证失败
    #[allow(dead_code)]
    pub fn auth<M>(msg: M) -> Self
    where
        M: Display,
    {
        let mut msg: String = format!("{msg}");
        msg.is_empty().then(|| msg.push_str("身份认证失败"));

        Self {
            code: StatusCode::UNAUTHORIZED.as_u16(),
            msg,
            data: None,
        }
    }

    ///  401 权限不足
    #[allow(dead_code)]
    pub fn privilege() -> Self {
        Self {
            code: StatusCode::UNAUTHORIZED.as_u16(),
            msg: "权限不足".to_owned(),
            data: None,
        }
    }

    /// 422 数据验证失败
    /// ## default: 数据验证失败
    #[allow(dead_code)]
    pub fn validate_failed<M: Display>(msg: M) -> Self
    where
        M: Display,
    {
        let mut msg: String = format!("{msg}");
        msg.is_empty().then(|| msg.push_str("数据验证失败"));

        Self {
            code: StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
            msg,
            data: None,
        }
    }

    /// 422 数据验证失败
    /// ### default msg: 数据验证失败
    #[allow(dead_code)]
    pub fn validate_failed_data(data: T) -> Self {
        Self {
            code: StatusCode::UNPROCESSABLE_ENTITY.as_u16(),
            msg: "数据验证失败".into(),
            data: Some(data),
        }
    }

    /// 403 服务拒绝
    /// ### default msg: 拒绝访问
    pub fn reject<M>(msg: M) -> Self
    where
        M: Display,
    {
        let mut msg: String = format!("{msg}");
        msg.is_empty().then(|| msg.push_str("拒绝访问"));

        Self {
            code: StatusCode::FORBIDDEN.as_u16(),
            msg,
            data: None,
        }
    }

    /// 500 服务器内部错误
    pub fn internal_error<M>(msg: M) -> Self
    where
        M: Display,
    {
        Self {
            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            msg: format!("{msg}"),
            data: None,
        }
    }
}
