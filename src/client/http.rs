//! Thin HTTP client for the scry REST API.

use std::time::Duration;

use reqwest::{Client as HttpClient, Method, Response, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::config::{ScryConfig, ScrySecrets};
use crate::error::AppError;

const TIMEOUT: Duration = Duration::from_secs(30);
const TIMEZONE_HEADER: &str = "X-Scry-Timezone";

/// A thin client for `scry serve`.
pub struct Client {
    http: HttpClient,
    base_url: String,
    token: Option<String>,
    timezone: String,
}

impl Client {
    /// Build a client from configuration, with `--server` taking precedence.
    pub fn new(
        config: &ScryConfig,
        server: Option<&str>,
        secrets: &ScrySecrets,
    ) -> Result<Self, AppError> {
        let base_url = config.resolved_server_url(server)?;
        let http = HttpClient::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|error| AppError::Config(format!("failed to build HTTP client: {error}")))?;

        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: secrets.api_token(),
            timezone: system_timezone(),
        })
    }

    /// GET a JSON resource from the API.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
    ) -> Result<T, AppError> {
        let url = self.build_url(segments, query)?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .map_err(|error| self.transport_error(error))?;

        decode(response).await
    }

    /// POST a JSON body and decode the JSON response.
    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
        body: &B,
    ) -> Result<T, AppError> {
        self.send_json(Method::POST, segments, query, body).await
    }

    /// PUT a JSON body and decode the JSON response.
    pub async fn put_json<B: Serialize, T: DeserializeOwned>(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
        body: &B,
    ) -> Result<T, AppError> {
        self.send_json(Method::PUT, segments, query, body).await
    }

    /// DELETE a resource and decode the JSON response.
    pub async fn delete_json<T: DeserializeOwned>(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
    ) -> Result<T, AppError> {
        let url = self.build_url(segments, query)?;
        let response = self
            .request(Method::DELETE, url)
            .send()
            .await
            .map_err(|error| self.transport_error(error))?;

        decode(response).await
    }

    async fn send_json<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        segments: &[&str],
        query: &[(&str, &str)],
        body: &B,
    ) -> Result<T, AppError> {
        let url = self.build_url(segments, query)?;
        let response = self
            .request(method, url)
            .json(body)
            .send()
            .await
            .map_err(|error| self.transport_error(error))?;

        decode(response).await
    }

    fn build_url(&self, segments: &[&str], query: &[(&str, &str)]) -> Result<Url, AppError> {
        let mut url = Url::parse(&format!("{}/api/v1", self.base_url)).map_err(|error| {
            AppError::Config(format!("invalid server URL '{}': {error}", self.base_url))
        })?;

        {
            let mut path = url.path_segments_mut().map_err(|_| {
                AppError::Config(format!(
                    "server URL '{}' cannot be used as a base",
                    self.base_url
                ))
            })?;
            for segment in segments {
                path.push(segment);
            }
        }

        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query.iter().copied());
        }

        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        let mut request = self
            .http
            .request(method, url)
            .header(TIMEZONE_HEADER, &self.timezone);

        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        request
    }

    fn transport_error(&self, error: reqwest::Error) -> AppError {
        AppError::Internal(format!(
            "failed to reach the server at {}: {error}",
            self.base_url
        ))
    }
}

/// Decode a successful body, or turn the server's error document into an error.
async fn decode<T: DeserializeOwned>(response: Response) -> Result<T, AppError> {
    let status = response.status();

    if status.is_success() {
        return response.json::<T>().await.map_err(|error| {
            AppError::Internal(format!("failed to decode server response: {error}"))
        });
    }

    let body = response
        .json::<ApiErrorBody>()
        .await
        .unwrap_or_else(|_| ApiErrorBody {
            kind: "internal".to_string(),
            message: format!("server returned {status}"),
            details: None,
        });

    Err(AppError::Remote {
        kind: remote_kind(&body.kind),
        message: body.message,
        details: body.details,
    })
}

#[derive(Deserialize)]
struct ApiErrorBody {
    kind: String,
    message: String,
    #[serde(default)]
    details: Option<serde_json::Value>,
}

fn remote_kind(kind: &str) -> &'static str {
    match kind {
        "not_found" => "not_found",
        "conflict" => "conflict",
        "invalid" => "invalid",
        "config" => "config",
        _ => "internal",
    }
}

/// The system's IANA time zone, defaulting to UTC.
fn system_timezone() -> String {
    std::env::var("TZ")
        .ok()
        .filter(|timezone| !timezone.is_empty())
        .unwrap_or_else(|| "UTC".to_string())
}
