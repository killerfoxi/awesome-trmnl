use chrono::{FixedOffset, NaiveDateTime};
use log::{debug, error};
use maud::{html, Markup, PreEscaped};
use url::Url;

use crate::generator;

pub fn content(weather: &Weather) -> Markup {
    html! {
        div ."layout layout--col layout--center" {
            div {
                (weather.current.weather_code.as_img(96))
            }
            div ."flex flex--row flex--center-x flex--top" {
                div ."flex flex--col flex--center gap--xsmall" {
                        span ."title px--1" { (format!("{:1}", weather.current.temperature)) }
                        span ."description px--1" { (PreEscaped(iconify::svg!("wi:celsius", width =  "32px"))) }
                }
                div ."flex flex--col flex--center gap--xsmall" {
                        span ."title px--1" { (weather.current.humidity) }
                        span ."description px--1" { (PreEscaped(iconify::svg!("wi:humidity", width = "28px"))) }
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Request,
    Geo,
    NotFound,
}

impl From<geolocation::Error> for Error {
    fn from(value: geolocation::Error) -> Self {
        match value {
            geolocation::Error::Request => Error::Request,
            geolocation::Error::Geo => Error::Geo,
            geolocation::Error::NotFound => Error::NotFound,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(_: reqwest::Error) -> Self {
        Self::Request
    }
}

#[derive(serde::Deserialize)]
#[serde(from = "u8")]
pub enum WeatherCode {
    Unclear,
    Clear,
    MostlyClear,
    PartlyCloudy,
    Overcast,
    Fog,
    DrizzleLight,
    DrizzleModerate,
    DrizzleDense,
    RainSlight,
    RainModerate,
    RainHeavy,
    Thunderstorm,
}

impl WeatherCode {
    pub fn as_img(&self, width: u32) -> Markup {
        match self {
            WeatherCode::Unclear => todo!(),
            WeatherCode::Clear => {
                html! { (PreEscaped(iconify::svg!("wi:day-sunny", width = "96px"))) }
            }
            WeatherCode::MostlyClear => {
                html! { (PreEscaped(iconify::svg!("wi:day-sunny-overcast", width = "96px"))) }
            }
            WeatherCode::PartlyCloudy => {
                html! { (PreEscaped(iconify::svg!("wi:day-cloudy", width = "96px"))) }
            }
            WeatherCode::Overcast => {
                html! { (PreEscaped(iconify::svg!("wi:cloudy", width =  "96px"))) }
            }
            WeatherCode::Fog => {
                html! { (PreEscaped(iconify::svg!("wi:day-fog", width = "96px"))) }
            }
            WeatherCode::DrizzleLight
            | WeatherCode::DrizzleModerate
            | WeatherCode::DrizzleDense => {
                html! { (PreEscaped(iconify::svg!("wi:day-sprinkle", width = "96px"))) }
            }
            WeatherCode::RainSlight | WeatherCode::RainModerate => {
                html! { (PreEscaped(iconify::svg!("wi:day-rain", width = "96px"))) }
            }
            WeatherCode::RainHeavy => {
                html! { (PreEscaped(iconify::svg!("wi:day-showers", width = "96px"))) }
            }
            WeatherCode::Thunderstorm => {
                html! { (PreEscaped(iconify::svg!("wi:day-thunderstorm", width = "96px"))) }
            }
        }
    }
}

impl From<u8> for WeatherCode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Clear,
            1 => Self::MostlyClear,
            2 => Self::PartlyCloudy,
            3 => Self::Overcast,
            45 | 48 => Self::Fog,
            51 => Self::DrizzleLight,
            53 => Self::DrizzleModerate,
            55 => Self::DrizzleDense,
            61 => Self::RainSlight,
            63 => Self::RainModerate,
            65 => Self::RainHeavy,
            95 => Self::Thunderstorm,
            _ => Self::Unclear,
        }
    }
}

mod incomplete_iso8601 {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &str = "%Y-%m-%dT%H:%M";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

mod utc_offset {
    use chrono::FixedOffset;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<FixedOffset, D::Error>
    where
        D: Deserializer<'de>,
    {
        let offset = i32::deserialize(deserializer)?;
        FixedOffset::east_opt(offset).ok_or(serde::de::Error::custom("Invalid offset"))
    }
}

#[derive(serde::Deserialize)]
pub struct Forecast {
    #[serde(with = "incomplete_iso8601")]
    time: NaiveDateTime,
    #[serde(rename = "temperature_2m")]
    temperature: f64,
    #[serde(rename = "relative_humidity_2m")]
    humidity: u8,
    weather_code: WeatherCode,
}

#[derive(serde::Deserialize)]
pub struct Weather {
    #[serde(rename = "utc_offset_seconds", with = "utc_offset")]
    ufc_offset: FixedOffset,
    current: Forecast,
}

pub struct Client {
    url: Url,
}

impl Client {
    pub async fn new(location: impl AsRef<str>) -> Result<Self, Error> {
        let mut url = Url::parse("https://api.open-meteo.com/v1/forecast").unwrap();
        let coords = geolocation::resolve(location).await?;
        url.query_pairs_mut()
            .clear()
            .append_pair("longitude", &format!("{:.2}", coords.0))
            .append_pair("latitude", &format!("{:.2}", coords.1));
        Ok(Self { url })
    }

    pub async fn fetch(&self) -> Result<Weather, reqwest::Error> {
        let mut url = self.url.clone();
        url.query_pairs_mut().append_pair(
            "current",
            "temperature_2m,relative_humidity_2m,weather_code",
        );
        reqwest::get(url)
            .await
            .inspect(|d| debug!("Got weather response: {d:#?}"))?
            .json()
            .await
    }

    pub async fn fetch_and_display(&self) -> Result<Markup, generator::Error> {
        let weather = self
            .fetch()
            .await
            .inspect_err(|e| error!("In weather data body: {e}"))?;
        Ok(content(&weather))
    }
}

mod geolocation {
    use http::header;
    use log::{debug, error};
    use serde::Deserialize;
    use url::Url;

    #[derive(Deserialize)]
    struct Geometry {
        coordinates: [f64; 2],
    }

    #[derive(Deserialize)]
    struct Feature {
        geometry: Geometry,
    }

    #[derive(Deserialize)]
    struct Response {
        features: Vec<Feature>,
    }

    pub enum Error {
        Request,
        Geo,
        NotFound,
    }

    pub async fn resolve(location: impl AsRef<str>) -> Result<(f64, f64), Error> {
        let mut search = Url::parse("https://nominatim.openstreetmap.org/search").unwrap();
        search
            .query_pairs_mut()
            .clear()
            .append_pair("city", location.as_ref())
            .append_pair("featureType", "settlement")
            .append_pair("format", "geojson");
        let resp: Response = reqwest::Client::new()
            .get(search)
            .header(header::USER_AGENT, "Awesome TRMNL")
            .send()
            .await
            .inspect_err(|e| error!("Fetching: {e}"))
            .inspect(|resp| debug!("Got response: {resp:#?}"))
            .map_err(|_| Error::Request)?
            .json()
            .await
            .inspect_err(|e| error!("Decoding: {e}"))
            .map_err(|_| Error::Geo)?;
        match resp.features.as_slice() {
            [] => Err(Error::NotFound),
            [first, ..] => Ok(first.geometry.coordinates.into()),
        }
    }
}
