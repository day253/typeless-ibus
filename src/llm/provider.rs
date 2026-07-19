use futures_util::future::BoxFuture;
use reqwest::header::{HeaderMap, RETRY_AFTER};
use serde_json::Value;
use std::fmt;
use std::time::Duration;

pub(crate) struct ProviderRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: u32,
}

#[derive(Debug, Default)]
pub(crate) struct ProviderUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct ProviderResponse {
    pub text: String,
    pub request_id: Option<String>,
    pub finish_reason: Option<String>,
    pub usage: ProviderUsage,
}

pub(crate) trait LlmProvider: Send + Sync {
    fn rewrite<'a>(
        &'a self,
        request: &'a ProviderRequest,
    ) -> BoxFuture<'a, Result<ProviderResponse, ProviderError>>;
}

#[derive(Debug)]
pub(crate) struct ProviderError {
    pub reason: &'static str,
    pub message: String,
    pub request_id: Option<String>,
}

impl ProviderError {
    pub fn new(reason: &'static str, message: impl Into<String>) -> Self {
        Self {
            reason,
            message: message.into(),
            request_id: None,
        }
    }

    pub fn with_request_id(mut self, request_id: Option<String>) -> Self {
        self.request_id = request_id;
        self
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ProviderError {}

pub(crate) struct JsonResponse {
    pub value: Value,
    pub request_id: Option<String>,
}

pub(crate) fn http_client(timeout: Duration) -> Result<reqwest::Client, ProviderError> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("typeless-ibus/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| ProviderError::new("client", format!("创建 LLM 网络客户端失败：{error}")))
}

pub(crate) async fn post_json_with_retry(
    client: &reqwest::Client,
    provider: &str,
    endpoint: &str,
    headers: &HeaderMap,
    body: &Value,
) -> Result<JsonResponse, ProviderError> {
    for attempt in 0..2_u32 {
        let response = client
            .post(endpoint)
            .headers(headers.clone())
            .json(body)
            .send()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                let reason = if error.is_timeout() {
                    "timeout"
                } else if error.is_connect() {
                    "connection"
                } else {
                    "network"
                };
                if attempt == 0 && (error.is_timeout() || error.is_connect()) {
                    tracing::warn!(provider, reason, retry_attempt = 1, "retrying LLM request");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
                return Err(ProviderError::new(reason, format!("LLM 请求失败：{error}")));
            }
        };

        let status = response.status();
        let response_headers = response.headers().clone();
        let request_id = extract_request_id(&response_headers);
        if !status.is_success() {
            let reason = match status.as_u16() {
                401 | 403 => "authentication",
                429 => "rate_limited",
                500..=599 => "server_error",
                _ => "request_rejected",
            };
            if attempt == 0 && (status.as_u16() == 429 || status.is_server_error()) {
                let delay = retry_delay(&response_headers);
                tracing::warn!(
                    provider,
                    status = status.as_u16(),
                    request_id = request_id.as_deref().unwrap_or("unknown"),
                    retry_attempt = 1,
                    retry_delay_ms = delay.as_millis() as u64,
                    "retrying rejected LLM request"
                );
                tokio::time::sleep(delay).await;
                continue;
            }
            return Err(ProviderError::new(
                reason,
                format!("LLM 接口返回 HTTP {}", status.as_u16()),
            )
            .with_request_id(request_id));
        }

        let value = response.json::<Value>().await.map_err(|error| {
            ProviderError::new("invalid_response", format!("解析 LLM 响应失败：{error}"))
                .with_request_id(request_id.clone())
        })?;
        return Ok(JsonResponse { value, request_id });
    }
    unreachable!("LLM retry loop always returns")
}

fn retry_delay(headers: &HeaderMap) -> Duration {
    headers
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.clamp(1, 5)))
        .unwrap_or_else(|| Duration::from_secs(1))
}

fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    [
        "x-request-id",
        "request-id",
        "x-trace-id",
        "x-tt-logid",
        "x-api-request-id",
    ]
    .into_iter()
    .find_map(|name| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[tokio::test]
    async fn retries_rate_limits_after_backoff() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request).unwrap();
                let response = if attempt == 0 {
                    "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 1\r\nX-Request-Id: limited-1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                } else {
                    let body = r#"{"ok":true}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Request-Id: success-2\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                };
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        let client = http_client(Duration::from_secs(5)).unwrap();
        let started_at = std::time::Instant::now();
        let response = post_json_with_retry(
            &client,
            "mock",
            &format!("http://{address}/rewrite"),
            &HeaderMap::new(),
            &serde_json::json!({"input": "test"}),
        )
        .await
        .unwrap();
        server.join().unwrap();
        assert!(started_at.elapsed() >= Duration::from_secs(1));
        assert_eq!(response.request_id.as_deref(), Some("success-2"));
        assert_eq!(response.value["ok"], true);
    }
}
