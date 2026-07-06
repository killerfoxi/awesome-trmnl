use std::time::Duration;

use chrono::{DateTime, Utc};
use log::{debug, error, warn};
use reqwest::{StatusCode, header, redirect};
use sailfish::TemplateOnce;
use url::Url;

use crate::generator;

#[derive(TemplateOnce)]
#[template(path = "ticktick.stpl")]
struct ContentTemplate<'a> {
    tasks: &'a [Task],
    now: DateTime<Utc>,
}

fn format_relative(deadline: DateTime<Utc>, now: DateTime<Utc>) -> String {
    use std::cmp::Ordering;
    let days = (deadline - now).num_days();
    match days.cmp(&0) {
        Ordering::Less => format!("{}d ago", days.abs()),
        Ordering::Equal => "today".into(),
        Ordering::Greater => format!("in {days}d"),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("the provided token is not a valid header value")]
    InvalidToken,
}

#[derive(Debug, thiserror::Error)]
pub enum FetchErrorKind {
    #[error("the request timed out")]
    Timeout,
    #[error("a connection error occurred")]
    Connection,
    #[error("the request was invalid")]
    InvalidRequest,
    #[error("permission was denied")]
    PermissionDenied,
    #[error("the resource was not found")]
    NotFound,
    #[error("authentication failed")]
    Unauthenticated,
    #[error("the response was not valid JSON")]
    Json,
    #[error("an unexpected error occurred")]
    Other,
}

#[allow(clippy::fallible_impl_from, reason = "we know it's a status")]
impl From<reqwest::Error> for FetchErrorKind {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() {
            Self::Connection
        } else if err.is_timeout() {
            Self::Timeout
        } else if err.is_request() {
            Self::InvalidRequest
        } else if err.is_status() {
            match err
                .status()
                .expect("is_status guarantees a status code is present")
            {
                StatusCode::NOT_FOUND => Self::NotFound,
                StatusCode::FORBIDDEN => Self::PermissionDenied,
                StatusCode::UNAUTHORIZED => Self::Unauthenticated,
                _ => Self::Other,
            }
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("fetching {} failed: {kind}", target.as_ref().map_or("<unknown>", Url::as_str))]
pub struct FetchError {
    pub kind: FetchErrorKind,
    pub target: Option<Url>,
}

impl From<reqwest::Error> for FetchError {
    fn from(value: reqwest::Error) -> Self {
        Self {
            target: value.url().cloned(),
            kind: value.into(),
        }
    }
}

impl From<FetchError> for generator::Error {
    fn from(err: FetchError) -> Self {
        let target = err
            .target
            .as_ref()
            .map_or_else(String::default, ToString::to_string);
        match err.kind {
            FetchErrorKind::Timeout => Self::Fetch {
                kind: generator::FetchErrorKind::Timeout,
                target,
            },
            FetchErrorKind::Connection => Self::Fetch {
                kind: generator::FetchErrorKind::Network,
                target,
            },
            FetchErrorKind::InvalidRequest => Self::Fetch {
                kind: generator::FetchErrorKind::Request(StatusCode::BAD_REQUEST),
                target,
            },
            FetchErrorKind::PermissionDenied => Self::Fetch {
                kind: generator::FetchErrorKind::Request(StatusCode::FORBIDDEN),
                target,
            },
            FetchErrorKind::NotFound => Self::Fetch {
                kind: generator::FetchErrorKind::Request(StatusCode::NOT_FOUND),
                target,
            },
            FetchErrorKind::Unauthenticated => Self::Fetch {
                kind: generator::FetchErrorKind::Request(StatusCode::UNAUTHORIZED),
                target,
            },
            FetchErrorKind::Json => Self::Fetch {
                kind: generator::FetchErrorKind::InvalidData,
                target,
            },
            FetchErrorKind::Other => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct Endpoint(Url);

impl Endpoint {
    pub fn for_project_data(&self, project: &Project) -> Url {
        self.0
            .join(&format!("project/{}/data", project.id))
            .expect("TickTick API path is always valid")
    }

    #[cfg(test)]
    fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Default for Endpoint {
    fn default() -> Self {
        Self(
            Url::parse("https://api.ticktick.com/open/v1/")
                .expect("Hardcoded TickTick URL is always valid"),
        )
    }
}

#[derive(serde::Deserialize)]
pub struct Auth {
    pub token: String,
    pub expires: Option<DateTime<Utc>>,
}

impl From<String> for Auth {
    fn from(value: String) -> Self {
        Self {
            token: value,
            expires: None,
        }
    }
}

impl From<&str> for Auth {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

pub struct Client {
    inner: reqwest::Client,
    endpoint: Endpoint,
}

impl Client {
    pub fn new<T: Into<Auth>>(auth: T) -> Result<Self, ClientError> {
        let auth: Auth = auth.into();
        if auth.expires.is_some_and(|e| e < Utc::now()) {
            warn!("Token might be expired!");
        }
        let mut token = header::HeaderValue::from_str(&format!("Bearer {}", auth.token))
            .map_err(|_| ClientError::InvalidToken)?;
        token.set_sensitive(true);
        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, token);
        Ok(Self {
            inner: reqwest::ClientBuilder::new()
                .redirect(redirect::Policy::none())
                .default_headers(headers)
                .build()
                .expect("Valid reqwest client configuration"),
            endpoint: Endpoint::default(),
        })
    }

    pub async fn fetch_tasks(&self, project: Project) -> Result<Box<[Task]>, FetchError> {
        debug!("Fetching data for {}", project.id);
        let pd: ProjectData = self
            .fetch(self.endpoint.for_project_data(&project))
            .await
            .inspect_err(|e| error!("While fetching: {e:?}"))?
            .json()
            .await
            .inspect_err(|e| error!("Converting into json: {e:?}"))?;
        Ok(pd.tasks.into_boxed_slice())
    }

    pub async fn fetch_and_display(&self, project: Project) -> Result<String, FetchError> {
        let now = Utc::now();
        Ok(content(&self.fetch_tasks(project).await?, now))
    }

    async fn fetch(&self, url: Url) -> Result<reqwest::Response, FetchError> {
        debug!("Fetching GET from: {url}");
        Ok(self
            .inner
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .error_for_status()?)
    }
}

#[derive(serde::Deserialize)]
struct ProjectData {
    tasks: Vec<Task>,
}

#[derive(Clone)]
pub struct Project {
    id: String,
}

impl From<String> for Project {
    fn from(id: String) -> Self {
        Self { id }
    }
}

#[derive(Debug, serde::Deserialize, Default)]
#[serde(from = "i32")]
pub enum Priority {
    None,
    Low,
    #[default]
    Medium,
    High,
}

impl From<i32> for Priority {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Low,
            3 => Self::Medium,
            5 => Self::High,
            _ => Self::None,
        }
    }
}

impl Priority {
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Medium => "iconoir-priority-medium",
            Self::High => "iconoir-priority-high",
            Self::Low => "iconoir-priority-down",
            Self::None => "",
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    title: String,
    #[serde(default)]
    content: String,
    due_date: Option<DateTime<Utc>>,
    start_date: Option<DateTime<Utc>>,
    priority: Priority,
}

pub fn content(tasks: &[Task], now: DateTime<Utc>) -> String {
    ContentTemplate { tasks, now }
        .render_once()
        .expect("ticktick template rendering failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_from_i32() {
        assert!(matches!(Priority::from(1), Priority::Low));
        assert!(matches!(Priority::from(3), Priority::Medium));
        assert!(matches!(Priority::from(5), Priority::High));
        assert!(matches!(Priority::from(0), Priority::None));
        assert!(matches!(Priority::from(99), Priority::None));
    }

    #[test]
    fn priority_icons() {
        assert_eq!(Priority::High.icon(), "iconoir-priority-high");
        assert_eq!(Priority::Medium.icon(), "iconoir-priority-medium");
        assert_eq!(Priority::Low.icon(), "iconoir-priority-down");
        assert_eq!(Priority::None.icon(), "");
    }

    #[test]
    fn auth_from_string() {
        let auth: Auth = "my_token".into();
        assert_eq!(auth.token, "my_token");
        assert!(auth.expires.is_none());
    }

    #[test]
    fn auth_from_str() {
        let auth: Auth = "my_token".into();
        assert_eq!(auth.token, "my_token");
    }

    #[test]
    fn endpoint_default() {
        let ep = Endpoint::default();
        assert_eq!(ep.as_str(), "https://api.ticktick.com/open/v1/");
    }

    #[test]
    fn endpoint_for_project_data() {
        let ep = Endpoint::default();
        let url = ep.for_project_data(&Project::from("proj123".to_string()));
        assert_eq!(
            url.as_str(),
            "https://api.ticktick.com/open/v1/project/proj123/data"
        );
    }

    #[test]
    fn content_renders_empty() {
        let now = Utc::now();
        let html = content(&[], now);
        assert!(html.contains("flex flex--left flex--row"));
    }

    #[test]
    fn content_renders_task() {
        let now = Utc::now();
        let task = Task {
            title: "Test".into(),
            content: "Details".into(),
            due_date: Some(now),
            start_date: Some(now),
            priority: Priority::High,
        };
        let html = content(&[task], now);
        assert!(html.contains("Test"));
        assert!(html.contains("Details"));
        assert!(html.contains("iconoir-priority-high"));
    }
}
