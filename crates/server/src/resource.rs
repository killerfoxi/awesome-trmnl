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
        Self::Local(
            format!("local:/content/{id}")
                .parse()
                .expect("Hardcoded local URL is always valid"),
        )
    }

    pub fn rendering(id: &str) -> Self {
        Self::Local(
            format!("local:/screen/{id}")
                .parse()
                .expect("Hardcoded local URL is always valid"),
        )
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
            Self::Local(path) => SELF_URL
                .get()
                .expect("Self URL not initialized")
                .join(path.path())
                .expect("Local path is always a valid URL suffix"),
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
            .expect("Hardcoded localhost URL is always valid"),
        )
        .expect("init_self called only once at startup");
}

pub fn self_url() -> Url {
    SELF_URL
        .get()
        .expect("Self URL not initialized")
        .clone()
}

static SELF_URL: OnceLock<Url> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_remote_http() {
        let res: Resource = "http://example.com/foo"
            .parse()
            .expect("Hardcoded URL is valid");
        assert!(matches!(res, Resource::Remote(url) if url.as_str() == "http://example.com/foo"));
    }

    #[test]
    fn parse_remote_https() {
        let res: Resource = "https://example.com/foo"
            .parse()
            .expect("Hardcoded URL is valid");
        assert!(matches!(res, Resource::Remote(url) if url.as_str() == "https://example.com/foo"));
    }

    #[test]
    fn parse_local_shorthand() {
        let res: Resource = "/content/abc".parse().expect("Hardcoded URL is valid");
        assert!(matches!(res, Resource::Local(url) if url.as_str() == "local:/content/abc"));
    }

    #[test]
    fn parse_local_explicit() {
        let res: Resource = "local:/screen/xyz"
            .parse()
            .expect("Hardcoded URL is valid");
        assert!(matches!(res, Resource::Local(url) if url.as_str() == "local:/screen/xyz"));
    }

    #[test]
    fn parse_unsupported_scheme() {
        let res: Result<Resource, _> = "ftp://example.com".parse();
        assert!(matches!(res, Err(Error::Unsupported)));
    }

    #[test]
    fn parse_invalid_url() {
        let res: Result<Resource, _> = "not a url".parse();
        assert!(matches!(res, Err(Error::InvalidFormat)));
    }

    #[test]
    fn self_hosted_content_creates_local() {
        let res = Resource::self_hosted_content("device1");
        assert!(matches!(res, Resource::Local(url) if url.path() == "/content/device1"));
    }

    #[test]
    fn rendering_creates_local() {
        let res = Resource::rendering("device1");
        assert!(matches!(res, Resource::Local(url) if url.path() == "/screen/device1"));
    }

    #[test]
    fn into_remote_converts_local() {
        let base: Url = "https://localhost:8080/"
            .parse()
            .expect("Hardcoded URL is valid");
        let res = Resource::self_hosted_content("d1")
            .into_remote(&base)
            .expect("Conversion succeeds");
        assert!(matches!(res, Resource::Remote(url) if url.as_str() == "https://localhost:8080/content/d1"));
    }

    #[test]
    fn into_remote_keeps_remote() {
        let base: Url = "https://localhost:8080/"
            .parse()
            .expect("Hardcoded URL is valid");
        let res = Resource::Remote(
            "https://example.com/foo"
                .parse()
                .expect("Hardcoded URL is valid"),
        )
        .into_remote(&base)
        .expect("Conversion succeeds");
        assert!(matches!(res, Resource::Remote(url) if url.as_str() == "https://example.com/foo"));
    }

    #[test]
    fn fully_qualified_url_remote() {
        let url: Url = "https://example.com/foo"
            .parse()
            .expect("Hardcoded URL is valid");
        let res = Resource::Remote(url.clone());
        assert_eq!(res.fully_qualified_url(), url);
    }

    #[test]
    fn as_href_local() {
        let res = Resource::Local("local:/content/abc".parse().expect("Hardcoded URL is valid"));
        assert_eq!(res.as_href(), "/content/abc");
    }

    #[test]
    fn as_href_remote() {
        let res = Resource::Remote(
            "https://example.com/foo"
                .parse()
                .expect("Hardcoded URL is valid"),
        );
        assert_eq!(res.as_href(), "https://example.com/foo");
    }

    #[test]
    fn init_self_sets_url() {
        init_self(8223, false);
        let url = self_url();
        assert_eq!(url.as_str(), "http://localhost:8223/");
    }
}
