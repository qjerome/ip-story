use rocket::{Request, response::Responder, serde::json::Json};
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

#[macro_export]
macro_rules! api_error {
    ($msg: expr) => {
        crate::api::ApiError::msg(format!("{}:{} {}", file!(), line!(), $msg))
    };
}

#[derive(ToSchema, Serialize)]
pub struct ApiResponse<D: Serialize> {
    error: Option<String>,
    data: Option<D>,
}

#[derive(ToSchema, Serialize)]
pub enum ApiData<D: Serialize> {
    Some(D),
    None,
}

impl<D> From<Option<D>> for ApiData<D>
where
    D: Serialize,
{
    fn from(value: Option<D>) -> Self {
        match value {
            Some(d) => ApiData::Some(d),
            None => ApiData::None,
        }
    }
}

impl<D> From<ApiData<D>> for Option<D>
where
    D: Serialize,
{
    fn from(value: ApiData<D>) -> Self {
        match value {
            ApiData::Some(d) => Some(d),
            ApiData::None => None,
        }
    }
}

impl<'r, D> Responder<'r, 'static> for ApiData<D>
where
    D: Serialize,
{
    fn respond_to(self, r: &'r Request<'_>) -> rocket::response::Result<'static> {
        let json = Json(ApiResponse {
            data: Option::<D>::from(self),
            error: None,
        });

        json.respond_to(r)
    }
}

pub type ApiResult<T> = Result<ApiData<T>, ApiError>;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    Msg(String),
}

impl ApiError {
    pub fn msg<S: AsRef<str>>(s: S) -> Self {
        ApiError::Msg(s.as_ref().to_string())
    }
}

// Implement the ResponseError trait for ApiError
impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, r: &'r Request<'_>) -> rocket::response::Result<'static> {
        let json = Json(ApiResponse::<()> {
            error: Some(self.to_string()),
            data: None,
        });

        json.respond_to(r)
    }
}
