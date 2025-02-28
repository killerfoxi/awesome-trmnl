use std::{borrow::Borrow, str::FromStr, sync::OnceLock};

use url::Url;

#[derive(Debug)]
pub enum Error {
    InvalidFormat,
    Unsupported,
}

#[derive(Debug, Clone)]
pub enum Resource {
    Local(Url),
    Remote(Url),
}

impl Resource {
    pub fn self_hosted_content(id: &str) -> Self {
        Self::Local(format!("local:/content/{id}").parse().unwrap())
    }

    pub fn rendering(id: &str) -> Self {
        Self::Local(format!("local:/screen/{id}").parse().unwrap())
    }

    pub fn into_remote(self, base: impl Borrow<Url>) -> Result<Self, Error> {
        match self {
            Self::Local(url) => Ok(Self::Remote(
                base.borrow()
                    .join(url.path())
                    .map_err(|_| Error::InvalidFormat)?,
            )),
            Self::Remote(url) => Ok(Self::Remote(url)),
        }
    }

    pub fn fully_qualified_url(&self) -> Url {
        match self {
            Self::Local(path) => SELF_URL.get().unwrap().join(path.path()).unwrap(),
            Self::Remote(u) => u.clone(),
        }
    }

    pub fn as_href(&self) -> &str {
        match self {
            Self::Local(l) => l.path(),
            Self::Remote(r) => r.as_str(),
        }
    }
}

impl FromStr for Resource {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url: Url = {
            if s.starts_with('/') {
                format!("local:{s}").parse()
            } else {
                s.parse()
            }
            .map_err(|_| Error::InvalidFormat)?
        };
        match url.scheme() {
            "http" | "https" => Ok(Self::Remote(url)),
            "local" => Ok(Self::Local(url)),
            _ => Err(Error::Unsupported),
        }
    }
}

pub fn init_self(port: u16, ssl: bool) {
    SELF_URL
        .set(
            Url::parse(&format!(
                "{}://localhost:{port}/",
                if ssl { "https" } else { "http" }
            ))
            .unwrap(),
        )
        .unwrap();
}

pub fn self_url() -> Url {
    SELF_URL.get().unwrap().clone()
}

static SELF_URL: OnceLock<Url> = OnceLock::new();
