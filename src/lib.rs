use base64::encode;
use cached::proc_macro::cached;
use jwfetch::{
    request, Duration, FetchError, HeaderMap, HeaderValue, HttpError, Method, RequestConfig,
    StatusCode,
};
use log::debug;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct GetAccessTokenOptions {
    pub client_id: String,
    pub client_secret: String,
    pub scope: String,
    pub user_agent: String,
    pub auth_api: String,
}

#[derive(Debug, Clone)]
pub struct GetOpenAPIOptions {
    pub client_id: String,
    pub user_agent: String,
    pub url: String,
    pub method: Method,
    /// Basic, Bearer
    pub authorization_type: String,
    pub body: Option<String>,
    pub auth_api: String,
    pub timeout: Option<Duration>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct BaseResult<T> {
    code: String,
    request_id: String,
    data: Option<T>,
    success: bool,
    message: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct AccessToken {
    access_token: String,
    expires_in: i32,
    /// "openid" | "read_userinfo" | "read_userinfo_by_phone" | "mmd_id" | "wecom"
    scope: Vec<String>,
    created_at: i32,
    state: String,
}

#[cached(time = 3600, result = true)]
pub async fn get_access_token(params: GetAccessTokenOptions) -> Result<String, FetchError> {
    let state = nanoid!();
    let url = format!(
        "/oauth/get_token?client_id={}&client_secret={}&state={}&scope={}",
        params.client_id, params.client_secret, state, params.scope
    );
    let mut headers = HeaderMap::new();
    headers.insert("user-agent", params.user_agent.parse().unwrap());
    let result = request::<BaseResult<AccessToken>>(RequestConfig {
        url,
        method: Method::GET,
        base_url: Some(params.auth_api),
        origin_headers: None,
        headers: Some(headers),
        data: None,
        timeout: None,
        extra_header_keys: None,
    })
    .await?;
    let data = trans_result(result)?;
    let access_token = data.access_token;
    debug!("Got access token: {}", access_token);
    Ok(access_token)
}

fn trans_result<T>(result: BaseResult<T>) -> Result<T, FetchError> {
    if result.success {
        Ok(result.data.unwrap())
    } else {
        Err(FetchError::Http(HttpError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("{}: {}", result.code, result.message.unwrap_or_default()),
        }))
    }
}

pub async fn get_open_api<T>(
    access_token: String,
    options: GetOpenAPIOptions,
) -> Result<T, FetchError>
where
    T: Serialize,
    for<'de2> T: Deserialize<'de2>,
{
    let base64 = format!("{}:{}", options.client_id, access_token);
    let authorization = format!("Basic {}", encode(base64));
    let mut headers = HeaderMap::new();
    headers.insert(
        "user-agent",
        HeaderValue::from_str(&options.user_agent).unwrap(),
    );
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&authorization).unwrap(),
    );
    let result = request::<BaseResult<T>>(RequestConfig {
        url: options.url,
        method: options.method,
        base_url: Some(options.auth_api),
        headers: Some(headers),
        data: options.body,
        timeout: options.timeout,
        origin_headers: None,
        extra_header_keys: None,
    })
    .await?;
    trans_result(result)
}

pub async fn get_auto_open_api<T>(
    access_token_options: GetAccessTokenOptions,
    openapi_options: GetOpenAPIOptions,
) -> Result<T, FetchError>
where
    T: Serialize,
    for<'de2> T: Deserialize<'de2>,
{
    let access_token = get_access_token(access_token_options).await?;
    get_open_api::<T>(access_token, openapi_options).await
}
