use std::{fmt::Display, ops::Deref};

use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use log::{debug, error};
use maud::{html, Markup};
use reqwest::{header, redirect};
use url::Url;

#[derive(Debug)]
pub enum ClientError {
    InvalidToken,
}

#[derive(Debug)]
pub enum FetchError {
    InvalidPath,
    Timeout,
    Connection,
    InvalidRequest,
    Other,
}

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() {
            Self::Connection
        } else if err.is_timeout() {
            Self::Timeout
        } else if err.is_request() {
            Self::InvalidRequest
        } else {
            Self::Other
        }
    }
}

#[derive(Debug)]
struct Endpoint(Url);

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

pub struct Client {
    inner: reqwest::Client,
    endpoint: Endpoint,
}

impl Client {
    pub fn new<T: Display>(auth: T) -> Result<Self, ClientError> {
        let mut token = header::HeaderValue::from_str(&format!("Bearer {auth}"))
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
            .fetch(&format!("/project/{}/data", project.id))
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

    async fn fetch(&self, path: &str) -> Result<reqwest::Response, FetchError> {
        let url = self
            .endpoint
            .join(path.strip_prefix("/").unwrap_or(path))
            .map_err(|_| FetchError::InvalidPath)?;
        debug!("Fetching GET from: {url}");
        Ok(self.inner.get(url).send().await?)
    }
}

#[derive(serde::Deserialize)]
struct ProjectData {
    tasks: Vec<Task>,
}

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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    id: String,
    title: String,
    #[serde(default)]
    content: String,
    due_date: Option<DateTime<Utc>>,
    start_date: Option<DateTime<Utc>>,
    priority: Priority,
}

pub fn content(tasks: &[Task], now: DateTime<Utc>) -> Markup {
    html! {
        div ."view view--full" {
            div ."layout layout--col layout--stretch-x" {
                (status_bar())
                div ."border--h-1" {}
                div .stretch {
                    (todos(tasks, now))
                }
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
                    @if let Some(start) = start {
                        (text_with_icon_and_modifier("timer", &start, "label--small label--inverted"))
                    }
                    @if let Some(due) = due {
                        (text_with_icon_and_modifier("timer-off", &due, "label--small label--inverted"))
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
            span .{"iconoir-" (icon)} {}
            span .label .{(modifier)} { (text) }
        }
    }
}

fn status_bar() -> Markup {
    let now = chrono::offset::Local::now();
    html! {
        div ."flex flex--left flex--row" {
            (text_with_icon("refresh", &format!("{}", now.format("%Y-%m-%d %H:%M:%S"))))
            div ."stretch-y" {
                div ."flex flex--row flex--right gap--medium" {
                    (text_with_icon("temperature-high", "23°C"))
                    (text_with_icon("droplet", "65%"))
                }
            }
        }
    }
}
