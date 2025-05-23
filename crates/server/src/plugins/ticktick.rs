use std::{ops::Deref, time::Duration};

use chrono::{DateTime, Utc};
use log::{debug, error, warn};
use maud::{Markup, html};
use reqwest::{StatusCode, header, redirect};
use url::Url;

use crate::generator;

#[derive(Debug)]
pub enum ClientError {
    InvalidToken,
}

#[derive(Debug)]
pub enum FetchErrorKind {
    Timeout,
    Connection,
    InvalidRequest,
    PermissionDenied,
    NotFound,
    Unauthenticated,
    Json,
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
            match err.status().unwrap() {
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

#[derive(Debug)]
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
            .unwrap()
    }
}

impl Default for Endpoint {
    fn default() -> Self {
        Self(Url::parse("https://api.ticktick.com/open/v1/").unwrap())
    }
}

impl Deref for Endpoint {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
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
        if let Some(expires) = auth.expires {
            if expires < Utc::now() {
                warn!("Token might be expired!");
            }
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
                .unwrap(),
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

    pub async fn fetch_and_display(&self, project: Project) -> Result<Markup, FetchError> {
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
    pub const fn icon(&self) -> &str {
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

pub fn content(tasks: &[Task], now: DateTime<Utc>) -> Markup {
    html! {
        div ."layout layout--col layout--stretch-x" {
            (status_bar(tasks.len()))
            div ."border--h-1" {}
            div .stretch {
                (todos(tasks, now))
            }
        }
    }
}

fn todos(tasks: &[Task], now: DateTime<Utc>) -> Markup {
    html! {
        div ."flex flex--left flex--col" {
            @for task in tasks {
                (entry(task, now))
            }
        }
    }
}

fn entry(task: &Task, now: DateTime<Utc>) -> Markup {
    use std::cmp::Ordering;

    let into_duration = |dl: DateTime<Utc>| {
        let dur = dl - now;
        match dur.num_days().cmp(&0) {
            Ordering::Less => format!("{}d ago", dur.num_days().abs()),
            Ordering::Equal => "today".into(),
            Ordering::Greater => format!("in {}d", dur.num_days()),
        }
    };
    let start = task.start_date.map(into_duration);
    let due = task.due_date.map(into_duration);
    html! {
        div .item {
            div .meta {}
            div .content {
                span ."title title--small" { (task.title) }
                @if !task.content.is_empty() {
                    span ."description" { (task.content) }
                }
                div ."flex flex--row gap" {
                    span .{(task.priority.icon())} {}
                    @if let Some(start) = start {
                        (text_with_icon_and_modifier("schedule", &start, "label--small label--inverted"))
                    }
                    @if let Some(due) = due {
                        (text_with_icon_and_modifier("alarm", &due, "label--small label--inverted"))
                    }
                }
            }
        }
    }
}

fn text_with_icon(icon: &str, text: &str) -> Markup {
    text_with_icon_and_modifier(icon, text, "")
}

fn text_with_icon_and_modifier(icon: &str, text: &str, modifier: &str) -> Markup {
    html! {
        div ."flex flex--row gap--small" {
            span ."material-symbols-outlined" { (icon) }
            span .label .{(modifier)} { (text) }
        }
    }
}

fn status_bar(num_tasks: usize) -> Markup {
    let now = chrono::offset::Local::now();
    html! {
        div ."flex flex--left flex--row" {
            (text_with_icon("update", &format!("{}", now.format("%Y-%m-%d %H:%M:%S"))))
            div ."stretch-y" {
                div ."flex flex--row flex--right gap--medium" {
                    (text_with_icon("numbers", &num_tasks.to_string()))
                }
            }
        }
    }
}
