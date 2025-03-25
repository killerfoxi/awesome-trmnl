use std::fmt::Display;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};
use itertools::izip;
use log::{debug, error};
use maud::{Markup, PreEscaped, html};
use url::Url;

use crate::generator;

#[derive(serde::Deserialize, Default, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Detail {
    Minimal,
    #[default]
    Full,
}

impl Detail {
    pub fn produce(&self, weather: &Weather) -> Markup {
        match self {
            Self::Minimal => minimal_content(weather),
            Self::Full => full_content(weather),
        }
    }
}

pub fn full_content(weather: &Weather) -> Markup {
    html! {
        div ."view view--full" {
            div ."layout layout--col gap--space-between" {
                div ."grid" {
                    div ."row row--center col--span-3 col--end" {
                        (weather.current.weather_code.as_img())
                    }
                    div ."col col--span-3 col--end" {
                        div ."item h--full" {
                            div ."meta" {}
                            div ."content" {
                                span ."value value--xxxlarge" data-fit-value="true" { (weather.current.temperature) "°" }
                                span ."label w--full" {
                                    (PreEscaped(iconify::svg!("wi:sunrise", width = "24px")))
                                    (weather.daily[0].sunrise.format("%H:%M"))
                                }
                                span ."label w--full" {
                                    (PreEscaped(iconify::svg!("wi:strong-wind", width = "24px")))
                                    (weather.daily[0].wind_dir.as_img())
                                    (weather.daily[0].wind_gusts)
                                }
                            }
                        }
                    }
                    div ."col col--span-3 col--end gap--medium" {
                        div ."item" {
                            div ."meta" {}
                            div ."icon" {
                                (PreEscaped(iconify::svg!("wi:thermometer")))
                            }
                            div ."content" {
                                span ."value value--small" { (weather.current.feels_like) "°" }
                                span ."label" { "Feels like" }
                            }
                        }

                        div ."item" {
                            div ."meta" {}
                            div ."icon" {
                                (PreEscaped(iconify::svg!("wi:raindrops", width = "24px")))
                            }
                            div ."content" {
                                span ."value value--small" { (weather.current.humidity) "%" }
                                span ."label" { "Humidity" }
                            }
                        }

                        div ."item" {
                            div ."meta" {}
                            div ."icon" {
                                (weather.current.weather_code.as_img())
                            }
                            div ."content" {
                                span ."value value--xsmall" { (weather.current.weather_code) }
                                span ."label" { "Right now" }
                            }
                        }
                    }
                }

                div ."w-full b-h-gray-5" {}

                div ."grid" {
                    div ."col gap--large" {
                    @for (i, day) in weather.daily.iter().enumerate().take(2) {
                        div ."grid" {
                        div ."item col--span-3" {
                            div ."meta" {}
                            div ."icon" {
                                (day.weather_code.as_img())
                            }
                            div ."content" {
                            span ."value value--xsmall" { (day.weather_code) }
                            span ."label" { @if i == 0 { "Today" } @else { "Tomorrow" } }
                            }
                        }

                        div ."row col--span-3" {
                            div ."item" {
                                div ."meta" {}
                                div ."row" {
                                    div ."icon" {
                                        (PreEscaped(iconify::svg!("wi:hot", width = "24px")))
                                    }

                                    div ."content w--14" {
                                        span ."value value--xsmall" { (day.uv_index) }
                                        span ."label" { "UV" }
                                    }

                                    div ."icon" style="margin-top: auto; margin-bottom: auto;" {
                                        (PreEscaped(iconify::svg!("wi:raindrop", width = "24px")))
                                    }

                                    div ."content w--14" style="justify-content: center" {
                                        span ."value value--xsmall" { "XX" "mm"}
                                        span ."label" { "Rain amount"}
                                    }
                                }
                            }
                        }

                        div ."row col--span-3" {
                            div ."item" {
                                div ."meta" {}
                                div ."icon" {
                                    (PreEscaped(iconify::svg!("wi:thermometer", width = "24px")))
                                }
                                div ."row" {
                                    div ."content w--20" {
                                        span ."value value--small" { (day.temperatures.min()) "°"}
                                        span ."label" { "Min" }
                                    }
                                    div ."content w--20" {
                                        span ."value value--small" { (day.temperatures.max()) "°"}
                                        span ."label" { "Max" }
                                    }
                                }
                            }
                        }
                        }
                    }
                    }
                }
            }
        }
    }
}

pub fn minimal_content(weather: &Weather) -> Markup {
    html! {
        div ."layout layout--col gap--space-between" {
            div {
                (weather.current.weather_code.as_img())
            }
            div ."grid row--center" {
                div ."col col--center col--span-2 w--full text--center" {
                        span ."value value--large" { (weather.current.temperature) }
                        span ."label w--full" { (PreEscaped(iconify::svg!("wi:celsius", width =  "32px"))) }
                }
                div ."col col--center col--span-2 w--full text--center" {
                        span ."value value--large" { (weather.current.humidity) }
                        span ."label w--full" { (PreEscaped(iconify::svg!("wi:humidity", width =  "32px"))) }
                }
            }
            div .grid {
                div ."row row--center gap--medium" {
                    div .item {
                        div .meta {}
                        div .content {
                            span ."value value--xxsmall" { (weather.daily[0].temperatures.min()) }
                            span ."description w--auto" { "min" }
                        }
                    }
                    div .item {
                        div .meta {}
                        div .content {
                            span ."value value--xxsmall" { (weather.daily[0].temperatures.max()) }
                            span ."description w--auto" { "max" }
                        }
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
            geolocation::Error::Request => Self::Request,
            geolocation::Error::Geo => Self::Geo,
            geolocation::Error::NotFound => Self::NotFound,
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
            Self::Unclear => {
                html! { (PreEscaped(iconify::svg!("wi:stars", width = "96px")))}
            }
            Self::Clear => {
                html! { (PreEscaped(iconify::svg!("wi:day-sunny", width = "96px"))) }
            }
            Self::MostlyClear => {
                html! { (PreEscaped(iconify::svg!("wi:day-sunny-overcast", width = "96px"))) }
            }
            Self::PartlyCloudy => {
                html! { (PreEscaped(iconify::svg!("wi:day-cloudy", width = "96px"))) }
            }
            Self::Overcast => {
                html! { (PreEscaped(iconify::svg!("wi:cloudy", width =  "96px"))) }
            }
            Self::Fog => {
                html! { (PreEscaped(iconify::svg!("wi:day-fog", width = "96px"))) }
            }
            Self::DrizzleLight | Self::DrizzleModerate | Self::DrizzleDense => {
                html! { (PreEscaped(iconify::svg!("wi:day-sprinkle", width = "96px"))) }
            }
            Self::RainSlight | Self::RainModerate => {
                html! { (PreEscaped(iconify::svg!("wi:day-rain", width = "96px"))) }
            }
            Self::RainHeavy => {
                html! { (PreEscaped(iconify::svg!("wi:day-showers", width = "96px"))) }
            }
            Self::Thunderstorm => {
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
            61 | 80 | 85 => Self::RainSlight,
            63 | 81 => Self::RainModerate,
            65 | 82 | 86 => Self::RainHeavy,
            95 => Self::Thunderstorm,
            _ => Self::Unclear,
        }
    }
}

impl Display for WeatherCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Clear => "Sunny",
                Self::MostlyClear => "Mostly Sunny",
                Self::PartlyCloudy => "Partially Cloudy",
                Self::Overcast => "Overcast",
                Self::Fog => "Foggy",
                Self::DrizzleLight => "Light Drizzle",
                Self::DrizzleModerate => "Moderate Drizzle",
                Self::DrizzleDense => "Dense Drizzle",
                Self::RainSlight => "Light Rain",
                Self::RainModerate => "Moderate Rain",
                Self::RainHeavy => "Heavy Rain",
                Self::Thunderstorm => "Thunderstorm",
                Self::Unclear => "Unclear",
            }
        )
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
        FixedOffset::east_opt(offset).ok_or_else(|| serde::de::Error::custom("Invalid offset"))
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
    pub const fn new(low: Temperature, high: Temperature) -> Self {
        Self(low, high)
    }

    pub const fn min(&self) -> &Temperature {
        &self.0
    }

    pub const fn max(&self) -> &Temperature {
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

#[derive(Debug, serde::Deserialize)]
#[serde(from = "u16")]
pub enum WindDirection {
    NorthWest,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
}

impl From<u16> for WindDirection {
    fn from(value: u16) -> Self {
        match value {
            20..70 => Self::NorthEast,
            70..110 => Self::East,
            110..160 => Self::SouthEast,
            160..200 => Self::South,
            200..250 => Self::SouthWest,
            250..290 => Self::West,
            290..340 => Self::NorthWest,
            _ => Self::North,
        }
    }
}

impl WindDirection {
    pub fn as_img(&self) -> Markup {
        match self {
            Self::NorthWest => {
                html! { (PreEscaped(iconify::svg!("wi:direction-up-left", width = "24px"))) }
            }
            Self::North => {
                html! { (PreEscaped(iconify::svg!("wi:direction-up", width = "24px"))) }
            }
            Self::NorthEast => {
                html! { (PreEscaped(iconify::svg!("wi:direction-up-right", width = "24px"))) }
            }
            Self::East => {
                html! { (PreEscaped(iconify::svg!("wi:direction-right", width = "24px"))) }
            }
            Self::SouthEast => {
                html! { (PreEscaped(iconify::svg!("wi:direction-down-right", width = "24px"))) }
            }
            Self::South => {
                html! { (PreEscaped(iconify::svg!("wi:direction-down", width = "24px"))) }
            }
            Self::SouthWest => {
                html! { (PreEscaped(iconify::svg!("wi:direction-down-left", width = "24px"))) }
            }
            Self::West => {
                html! { (PreEscaped(iconify::svg!("wi:direction-left", width = "24px"))) }
            }
        }
    }
}

pub struct CurrentForecast {
    pub time: DateTime<FixedOffset>,
    pub temperature: Temperature,
    pub feels_like: Temperature,
    pub humidity: Humidity,
    pub weather_code: WeatherCode,
}

pub struct DailyForecast {
    pub date: NaiveDate,
    pub temperatures: TemperatureRange,
    pub weather_code: WeatherCode,
    pub sunrise: NaiveDateTime,
    pub uv_index: f64,
    pub wind_speed: f64,
    pub wind_gusts: f64,
    pub wind_dir: WindDirection,
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
                feels_like: current.feels_like.into(),
                humidity: current.humidity.into(),
                weather_code: current.weather_code,
            },
            daily: izip!(
                daily.time,
                daily.temperature_max,
                daily.temperature_min,
                daily.weather_code,
                daily.sunrise,
                daily.uv_index,
                daily.wind_speeds,
                daily.wind_gusts,
                daily.wind_dirs,
            )
            .map(
                |(time, tmax, tmin, wc, sunrise, uv_index, wind_speed, wind_gusts, wind_dir)| {
                    DailyForecast {
                        date: time,
                        temperatures: TemperatureRange::new(tmin.into(), tmax.into()),
                        weather_code: wc,
                        sunrise: sunrise.into_inner(),
                        uv_index,
                        wind_speed,
                        wind_gusts,
                        wind_dir,
                    }
                },
            )
            .collect(),
        })
    }
}

mod intermediate {
    use std::ops::Deref;

    use chrono::{FixedOffset, NaiveDate, NaiveDateTime};

    use super::{WeatherCode, WindDirection};

    #[derive(serde::Deserialize)]
    #[serde(transparent)]
    pub struct DayAndTime(#[serde(with = "super::incomplete_iso8601")] NaiveDateTime);

    impl DayAndTime {
        pub const fn into_inner(self) -> NaiveDateTime {
            self.0
        }
    }

    impl Deref for DayAndTime {
        type Target = NaiveDateTime;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[derive(serde::Deserialize)]
    pub struct CurrentForecast {
        pub time: DayAndTime,
        #[serde(rename = "temperature_2m")]
        pub temperature: f64,
        #[serde(rename = "apparent_temperature")]
        pub feels_like: f64,
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
        pub sunrise: Vec<DayAndTime>,
        #[serde(rename = "uv_index_max")]
        pub uv_index: Vec<f64>,
        #[serde(rename = "wind_speed_10m_max")]
        pub wind_speeds: Vec<f64>,
        #[serde(rename = "wind_gusts_10m_max")]
        pub wind_gusts: Vec<f64>,
        #[serde(rename = "wind_direction_10m_dominant")]
        pub wind_dirs: Vec<WindDirection>,
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
    detail: Detail,
}

impl Client {
    pub async fn new(location: impl AsRef<str>, detail: Detail) -> Result<Self, Error> {
        let mut url = Url::parse("https://api.open-meteo.com/v1/forecast").unwrap();
        let coords = geolocation::resolve(location).await?;
        url.query_pairs_mut()
            .clear()
            .append_pair("longitude", &format!("{:.2}", coords.0))
            .append_pair("latitude", &format!("{:.2}", coords.1));
        Ok(Self { url, detail })
    }

    pub async fn fetch(&self) -> Result<Weather, reqwest::Error> {
        let mut url = self.url.clone();
        url.query_pairs_mut()
            .append_pair(
                "current",
                "temperature_2m,relative_humidity_2m,weather_code,apparent_temperature,precipitation,rain,wind_speed_10m",
            )
            .append_pair(
                "daily",
                "weather_code,temperature_2m_max,temperature_2m_min,sunrise,uv_index_max,wind_speed_10m_max,wind_gusts_10m_max,wind_direction_10m_dominant",
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
        Ok(self.detail.produce(&weather))
    }
}

mod geolocation {
    use std::time::Duration;

    use http::header;
    use log::{debug, error};
    use serde::Deserialize;
    use url::Url;

    use crate::net;

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
        let resp: Response = net::retry(
            || {
                reqwest::Client::new()
                    .get(search.clone())
                    .header(header::USER_AGENT, "Awesome TRMNL")
                    .send()
            },
            Duration::from_millis(500),
            Duration::from_secs(2 * 60),
        )
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
