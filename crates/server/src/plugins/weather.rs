use std::fmt::Display;

use chrono::{DateTime, FixedOffset, NaiveDate};
use itertools::izip;
use log::{debug, error};
use maud::{html, Markup, PreEscaped};
use url::Url;

use crate::generator;

pub fn content(weather: &Weather) -> Markup {
    html! {
        div ."layout layout--col layout--center" {
            div {
                (weather.current.weather_code.as_img())
            }
            div ."flex flex--row flex--center-x flex--top gap" {
                div ."flex flex--col flex--center gap--xsmall" {
                        span ."title px--1" { (weather.current.temperature) }
                        span ."description px--1" { (PreEscaped(iconify::svg!("wi:celsius", width =  "32px"))) }
                }
                div ."flex flex--col flex--center gap--xsmall" {
                        span ."title px--1" { (weather.current.humidity) }
                        span ."description px--1" { (PreEscaped(iconify::svg!("wi:humidity", width = "28px"))) }
                }
            }
            div ."stretch-x flex flex--row flex--stretch-x flex--top gap" {
                div .item {
                    div .meta {}
                    div .content {
                        span ."title title--small" { (weather.daily[0].temperatures.min()) }
                        span ."description" { "min" }
                    }
                }
                div .item {
                    div .meta {}
                    div .content {
                        span ."title title--small" { (weather.daily[0].temperatures.max()) }
                        span ."description" { "max" }
                    }
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
    pub fn as_img(&self) -> Markup {
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

pub struct Temperature(f64);

impl Display for Temperature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}", self.0)
    }
}

impl From<f64> for Temperature {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

pub struct TemperatureRange(Temperature, Temperature);

impl TemperatureRange {
    pub fn new(low: Temperature, high: Temperature) -> Self {
        Self(low, high)
    }

    pub fn min(&self) -> &Temperature {
        &self.0
    }

    pub fn max(&self) -> &Temperature {
        &self.1
    }
}

pub struct Humidity(u8);

impl Display for Humidity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u8> for Humidity {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

pub struct CurrentForecast {
    pub time: DateTime<FixedOffset>,
    pub temperature: Temperature,
    pub humidity: Humidity,
    pub weather_code: WeatherCode,
}

pub struct DailyForecast {
    pub date: NaiveDate,
    pub temperatures: TemperatureRange,
    pub weather_code: WeatherCode,
}

#[derive(serde::Deserialize)]
#[serde(try_from = "intermediate::Weather")]
pub struct Weather {
    pub current: CurrentForecast,
    pub daily: Vec<DailyForecast>,
}

pub struct ConvertError;

impl Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to convert into target struct")
    }
}

impl TryFrom<intermediate::Weather> for Weather {
    type Error = ConvertError;

    fn try_from(weather: intermediate::Weather) -> Result<Self, Self::Error> {
        let (current, daily) = (weather.current, weather.daily);
        Ok(Self {
            current: CurrentForecast {
                time: current
                    .time
                    .and_local_timezone(weather.ufc_offset)
                    .latest()
                    .ok_or(ConvertError)?,
                temperature: current.temperature.into(),
                humidity: current.humidity.into(),
                weather_code: current.weather_code,
            },
            daily: izip!(
                daily.time,
                daily.temperature_max,
                daily.temperature_min,
                daily.weather_code
            )
            .map(|(time, tmax, tmin, wc)| DailyForecast {
                date: time,
                temperatures: TemperatureRange::new(tmin.into(), tmax.into()),
                weather_code: wc,
            })
            .collect(),
        })
    }
}

mod intermediate {
    use chrono::{FixedOffset, NaiveDate, NaiveDateTime};

    use super::WeatherCode;

    #[derive(serde::Deserialize)]
    pub struct CurrentForecast {
        #[serde(with = "super::incomplete_iso8601")]
        pub time: NaiveDateTime,
        #[serde(rename = "temperature_2m")]
        pub temperature: f64,
        #[serde(rename = "relative_humidity_2m")]
        pub humidity: u8,
        pub weather_code: WeatherCode,
    }

    #[derive(serde::Deserialize)]
    pub struct DailyForecast {
        pub time: Vec<NaiveDate>,
        #[serde(rename = "temperature_2m_max")]
        pub temperature_max: Vec<f64>,
        #[serde(rename = "temperature_2m_min")]
        pub temperature_min: Vec<f64>,
        pub weather_code: Vec<WeatherCode>,
    }

    #[derive(serde::Deserialize)]
    pub struct Weather {
        #[serde(rename = "utc_offset_seconds", with = "super::utc_offset")]
        pub ufc_offset: FixedOffset,
        pub current: CurrentForecast,
        pub daily: DailyForecast,
    }
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
        url.query_pairs_mut()
            .append_pair(
                "current",
                "temperature_2m,relative_humidity_2m,weather_code",
            )
            .append_pair(
                "daily",
                "weather_code,temperature_2m_max,temperature_2m_min",
            )
            .append_pair("forecast_days", "3");
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
