use chrono::{FixedOffset, NaiveDateTime};
use log::{debug, error};
use maud::{html, Markup};
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
                        span ."title px--1" { (weather.current.temperature) "°" }
                        span ."description px--1" { "Temperature" }
                }
                div ."flex flex--col flex--center gap--xsmall" {
                        span ."title px--1" { (weather.current.humidity) "%" }
                        span ."description px--1" { "Humidity" }
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
            WeatherCode::Clear | WeatherCode::MostlyClear => {
                html! { img src="data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiA/PjwhRE9DVFlQRSBzdmcgIFBVQkxJQyAnLS8vVzNDLy9EVEQgU1ZHIDEuMS8vRU4nICAnaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkJz48c3ZnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXcgMCAwIDUxMiA1MTIiIGhlaWdodD0iNTEycHgiIGlkPSJMYXllcl8xIiB2ZXJzaW9uPSIxLjEiIHZpZXdCb3g9IjAgMCA1MTIgNTEyIiB3aWR0aD0iNTEycHgiIHhtbDpzcGFjZT0icHJlc2VydmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgeG1sbnM6eGxpbms9Imh0dHA6Ly93d3cudzMub3JnLzE5OTkveGxpbmsiPjxnPjxjaXJjbGUgY3g9IjI1NS44OTkiIGN5PSIyNTYuNDUiIGZpbGw9Im5vbmUiIHI9IjEwNS43MDYiIHN0cm9rZT0iIzAwMDAwMCIgc3Ryb2tlLW1pdGVybGltaXQ9IjEwIiBzdHJva2Utd2lkdGg9IjMwIi8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSIyNTUiIHgyPSIyNTUiIHkxPSI1OCIgeTI9IjExNSIvPjxsaW5lIGZpbGw9Im5vbmUiIHN0cm9rZT0iIzAwMDAwMCIgc3Ryb2tlLW1pdGVybGltaXQ9IjEwIiBzdHJva2Utd2lkdGg9IjMwIiB4MT0iNTYiIHgyPSIxMTQiIHkxPSIyNTYiIHkyPSIyNTYiLz48bGluZSBmaWxsPSJub25lIiBzdHJva2U9IiMwMDAwMDAiIHN0cm9rZS1taXRlcmxpbWl0PSIxMCIgc3Ryb2tlLXdpZHRoPSIzMCIgeDE9IjM5OSIgeDI9IjQ1NiIgeTE9IjI1NiIgeTI9IjI1NiIvPjxsaW5lIGZpbGw9Im5vbmUiIHN0cm9rZT0iIzAwMDAwMCIgc3Ryb2tlLW1pdGVybGltaXQ9IjEwIiBzdHJva2Utd2lkdGg9IjMwIiB4MT0iMjU1IiB4Mj0iMjU1IiB5MT0iMzk3IiB5Mj0iNDU0Ii8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSIzNTUuMDM0IiB4Mj0iMzk1LjgwNCIgeTE9IjE1OS4xMTgiIHkyPSIxMTguMzQ4Ii8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSIzNTUuMDM0IiB4Mj0iMzk1LjgwNCIgeTE9IjM1Ni4xODYiIHkyPSIzOTYuOTU2Ii8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSIxMDkuOTg5IiB4Mj0iMTUwLjc1OSIgeTE9IjM5Ni45NTYiIHkyPSIzNTYuMTg2Ii8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSIxMDkuOTg5IiB4Mj0iMTUwLjc1OSIgeTE9IjExOC4zNDgiIHkyPSIxNTkuMTE4Ii8+PC9nPjwvc3ZnPg==" width=(width); }
            }
            WeatherCode::PartlyCloudy => {
                html! { img src="data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiA/PjwhRE9DVFlQRSBzdmcgIFBVQkxJQyAnLS8vVzNDLy9EVEQgU1ZHIDEuMS8vRU4nICAnaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkJz48c3ZnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXcgMCAwIDUxMiA1MTIiIGhlaWdodD0iNTEycHgiIGlkPSJMYXllcl8xIiB2ZXJzaW9uPSIxLjEiIHZpZXdCb3g9IjAgMCA1MTIgNTEyIiB3aWR0aD0iNTEycHgiIHhtbDpzcGFjZT0icHJlc2VydmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgeG1sbnM6eGxpbms9Imh0dHA6Ly93d3cudzMub3JnLzE5OTkveGxpbmsiPjxnPjxwYXRoIGQ9Ik00NTYuMDg3LDM2Ny44MzkgICBjMCwyMS40NTItMTcuMzk2LDM5LjE2MS0zOC44MTQsMzkuMTYxSDIwOS4xMDljLTM0Ljc5MiwwLTYyLjQzMi0yOC41MzgtNjIuNDMyLTYzLjMzYzAtMTQuNjExLDQuOTg1LTI4LjA1NSwxMy4zNzQtMzguNzgxICAgYzExLjQ4My0xNC44MTcsMjkuNDI4LTI0LjM0MSw0OS42MDktMjQuMzQxYzMuMDI1LDAsNi4wMTYsMC4wODYsOC45MzksMC41MzNjNi4zMjYtMjUuNzUsMjIuNTE4LTQ3LjY0MSw0NC4zNDktNjEuMzU4ICAgYzE1LjUwNS05Ljc5OCwzMy44OTgtMTUuNDMyLDUzLjU2My0xNS40MzJjNTUuNjYsMCwxMDAuNzY1LDQ1LjEwNywxMDAuNzY1LDEwMC43NjdjMCw4LjE4Mi0wLjk2MywxNi4xMjUtMi44MTksMjMuNzU3ICAgYzAuOTI4LTAuMDY5LDEuODU3LDAuMDcxLDIuODE5LDAuMDcxQzQzOC42OTIsMzI4Ljg4NSw0NTYuMDg3LDM0Ni40Miw0NTYuMDg3LDM2Ny44Mzl6IiBmaWxsPSJub25lIiBzdHJva2U9IiMwMDAwMDAiIHN0cm9rZS1taXRlcmxpbWl0PSIxMCIgc3Ryb2tlLXdpZHRoPSIzMCIvPjxwYXRoIGQ9Ik0yNjIuOTQ2LDIxOS43MyAgIGMtMjEuODMxLDEzLjcxNy0zOC4wMjMsMzUuNjE3LTQ0LjM0OSw2MS4zNjZjLTIuOTIyLTAuNDQ3LTUuOTEzLTAuNTE2LTguOTM5LTAuNTE2Yy0yMC4xOCwwLTM4LjEyNiw5LjU1Ny00OS42MDksMjQuMzc1ICAgYy0yMy4wMzQtMTIuMTM2LTM4Ljc0NS0zNi4yNy0zOC43NDUtNjQuMTE3YzAtMzkuOTgzLDMyLjQyLTcyLjQwMiw3Mi40MDItNzIuNDAyQzIyNi4zNjcsMTY4LjQzNiwyNTMuOTM5LDE5MC4wMjcsMjYyLjk0NiwyMTkuNzN6ICAgIiBmaWxsPSJub25lIiBzdHJva2U9IiMwMDAwMDAiIHN0cm9rZS1taXRlcmxpbWl0PSIxMCIgc3Ryb2tlLXdpZHRoPSIzMCIvPjxsaW5lIGZpbGw9Im5vbmUiIHN0cm9rZT0iIzAwMDAwMCIgc3Ryb2tlLW1pdGVybGltaXQ9IjEwIiBzdHJva2Utd2lkdGg9IjMwIiB4MT0iMTk0IiB4Mj0iMTk0IiB5MT0iMTA1IiB5Mj0iMTQzIi8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSI1NiIgeDI9Ijk0IiB5MT0iMjQyIiB5Mj0iMjQyIi8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSIyNjEuNjIyIiB4Mj0iMjg5LjU0NyIgeTE9IjE3NC4xNzEiIHkyPSIxNDYuMjQ2Ii8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSI5My43ODIiIHgyPSIxMjEuNzA3IiB5MT0iMzM3LjA3NSIgeTI9IjMwOS4xNSIvPjxsaW5lIGZpbGw9Im5vbmUiIHN0cm9rZT0iIzAwMDAwMCIgc3Ryb2tlLW1pdGVybGltaXQ9IjEwIiBzdHJva2Utd2lkdGg9IjMwIiB4MT0iOTMuNzgyIiB4Mj0iMTIxLjcwNyIgeTE9IjE0Ni4yNDYiIHkyPSIxNzQuMTcxIi8+PC9nPjwvc3ZnPg==" width=(width); }
            }
            WeatherCode::Overcast => {
                html! { img src="data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiA/PjwhRE9DVFlQRSBzdmcgIFBVQkxJQyAnLS8vVzNDLy9EVEQgU1ZHIDEuMS8vRU4nICAnaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkJz48c3ZnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXcgMCAwIDUxMiA1MTIiIGhlaWdodD0iNTEycHgiIGlkPSJMYXllcl8xIiB2ZXJzaW9uPSIxLjEiIHZpZXdCb3g9IjAgMCA1MTIgNTEyIiB3aWR0aD0iNTEycHgiIHhtbDpzcGFjZT0icHJlc2VydmUiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgeG1sbnM6eGxpbms9Imh0dHA6Ly93d3cudzMub3JnLzE5OTkveGxpbmsiPjxnPjxwYXRoIGQ9Ik00NTYuMDg3LDM2Ny44MzkgICBjMCwyMS40NTItMTcuMzk2LDM5LjE2MS0zOC44MTQsMzkuMTYxSDIwOS4xMDljLTM0Ljc5MiwwLTYyLjQzMi0yOC41MzgtNjIuNDMyLTYzLjMzYzAtMTQuNjExLDQuOTg1LTI4LjA1NSwxMy4zNzQtMzguNzgxICAgYzExLjQ4My0xNC44MTcsMjkuNDI4LTI0LjM0MSw0OS42MDktMjQuMzQxYzMuMDI1LDAsNi4wMTYsMC4wODYsOC45MzksMC41MzNjNi4zMjYtMjUuNzUsMjIuNTE4LTQ3LjY0MSw0NC4zNDktNjEuMzU4ICAgYzE1LjUwNS05Ljc5OCwzMy44OTgtMTUuNDMyLDUzLjU2My0xNS40MzJjNTUuNjYsMCwxMDAuNzY1LDQ1LjEwNywxMDAuNzY1LDEwMC43NjdjMCw4LjE4Mi0wLjk2MywxNi4xMjUtMi44MTksMjMuNzU3ICAgYzAuOTI4LTAuMDY5LDEuODU3LDAuMDcxLDIuODE5LDAuMDcxQzQzOC42OTIsMzI4Ljg4NSw0NTYuMDg3LDM0Ni40Miw0NTYuMDg3LDM2Ny44Mzl6IiBmaWxsPSJub25lIiBzdHJva2U9IiMwMDAwMDAiIHN0cm9rZS1taXRlcmxpbWl0PSIxMCIgc3Ryb2tlLXdpZHRoPSIzMCIvPjxwYXRoIGQ9Ik0yNjIuOTQ2LDIxOS43MyAgIGMtMjEuODMxLDEzLjcxNy0zOC4wMjMsMzUuNjE3LTQ0LjM0OSw2MS4zNjZjLTIuOTIyLTAuNDQ3LTUuOTEzLTAuNTE2LTguOTM5LTAuNTE2Yy0yMC4xOCwwLTM4LjEyNiw5LjU1Ny00OS42MDksMjQuMzc1ICAgYy0yMy4wMzQtMTIuMTM2LTM4Ljc0NS0zNi4yNy0zOC43NDUtNjQuMTE3YzAtMzkuOTgzLDMyLjQyLTcyLjQwMiw3Mi40MDItNzIuNDAyQzIyNi4zNjcsMTY4LjQzNiwyNTMuOTM5LDE5MC4wMjcsMjYyLjk0NiwyMTkuNzN6ICAgIiBmaWxsPSJub25lIiBzdHJva2U9IiMwMDAwMDAiIHN0cm9rZS1taXRlcmxpbWl0PSIxMCIgc3Ryb2tlLXdpZHRoPSIzMCIvPjxsaW5lIGZpbGw9Im5vbmUiIHN0cm9rZT0iIzAwMDAwMCIgc3Ryb2tlLW1pdGVybGltaXQ9IjEwIiBzdHJva2Utd2lkdGg9IjMwIiB4MT0iMTk0IiB4Mj0iMTk0IiB5MT0iMTA1IiB5Mj0iMTQzIi8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSI1NiIgeDI9Ijk0IiB5MT0iMjQyIiB5Mj0iMjQyIi8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSIyNjEuNjIyIiB4Mj0iMjg5LjU0NyIgeTE9IjE3NC4xNzEiIHkyPSIxNDYuMjQ2Ii8+PGxpbmUgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjMDAwMDAwIiBzdHJva2UtbWl0ZXJsaW1pdD0iMTAiIHN0cm9rZS13aWR0aD0iMzAiIHgxPSI5My43ODIiIHgyPSIxMjEuNzA3IiB5MT0iMzM3LjA3NSIgeTI9IjMwOS4xNSIvPjxsaW5lIGZpbGw9Im5vbmUiIHN0cm9rZT0iIzAwMDAwMCIgc3Ryb2tlLW1pdGVybGltaXQ9IjEwIiBzdHJva2Utd2lkdGg9IjMwIiB4MT0iOTMuNzgyIiB4Mj0iMTIxLjcwNyIgeTE9IjE0Ni4yNDYiIHkyPSIxNzQuMTcxIi8+PC9nPjwvc3ZnPg==" width=(width); }
            }
            WeatherCode::Fog => {
                html! { img src="data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiA/PjwhRE9DVFlQRSBzdmcgIFBVQkxJQyAnLS8vVzNDLy9EVEQgU1ZHIDEuMS8vRU4nICAnaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkJz48c3ZnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXcgMCAwIDk2IDk2IiBoZWlnaHQ9Ijk2cHgiIHZlcnNpb249IjEuMSIgdmlld0JveD0iMCAwIDk2IDk2IiB3aWR0aD0iOTZweCIgeG1sOnNwYWNlPSJwcmVzZXJ2ZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB4bWxuczp4bGluaz0iaHR0cDovL3d3dy53My5vcmcvMTk5OS94bGluayI+PGcgaWQ9IkRpYnVqbyI+PGc+PHBhdGggZD0iTTIxLDUzYzEuMTA0LDAsMi0wLjg5NiwyLTJjMC0zLjg1OSwzLjE0MS03LDctN2MwLjI3NywwLDAuNzIzLDAuMDY4LDEuMTkzLDAuMTYyVjQ2YzAsMS4xMDQsMC44OTYsMiwyLDIgICAgYzEuMTA0LDAsMi0wLjg5NiwyLTJ2LTMuMjE5QzM2LjI2NiwzNi41MjgsNDEuNjI5LDMyLDQ4LDMyczExLjczNCw0LjUyOCwxMi44MDcsMTAuNzgxVjQ2YzAsMS4xMDQsMC44OTUsMiwyLDIgICAgYzEuMTA1LDAsMi0wLjg5NiwyLTJ2LTEuODM4QzY1LjI3Nyw0NC4wNjgsNjUuNzIzLDQ0LDY2LDQ0YzMuODU5LDAsNywzLjE0MSw3LDdjMCwxLjEwNCwwLjg5NiwyLDIsMnMyLTAuODk2LDItMiAgICBjMC02LjA2NS00LjkzNS0xMS0xMS0xMWMtMC41MDcsMC0xLjExMiwwLjA3OS0xLjY4OSwwLjE4NEM2Mi4yMTksMzMuMDEyLDU1LjY2NCwyOCw0OCwyOHMtMTQuMjE5LDUuMDEyLTE2LjMxMiwxMi4xODQgICAgQzMxLjExMiw0MC4wNzksMzAuNTA3LDQwLDMwLDQwYy02LjA2NSwwLTExLDQuOTM1LTExLDExQzE5LDUyLjEwNCwxOS44OTYsNTMsMjEsNTN6Ii8+PHBhdGggZD0iTTQwLDU5YzAsMS4xMDQsMC44OTUsMiwyLDJoMzZjMS4xMDQsMCwyLTAuODk2LDItMnMtMC44OTYtMi0yLTJINDJDNDAuODk1LDU3LDQwLDU3Ljg5Niw0MCw1OXoiLz48cGF0aCBkPSJNNDgsNjRjLTEuMTA1LDAtMiwwLjg5Ni0yLDJzMC44OTUsMiwyLDJoMjdjMS4xMDQsMCwyLTAuODk2LDItMnMtMC44OTYtMi0yLTJINDh6Ii8+PHBhdGggZD0iTTc4LDcxSDU0Yy0xLjEwNSwwLTIsMC44OTYtMiwyczAuODk1LDIsMiwyaDI0YzEuMTA0LDAsMi0wLjg5NiwyLTJTNzkuMTA0LDcxLDc4LDcxeiIvPjxwYXRoIGQ9Ik0xOCw2MWgxOGMxLjEwNCwwLDItMC44OTYsMi0ycy0wLjg5Ni0yLTItMkgxOGMtMS4xMDQsMC0yLDAuODk2LTIsMlMxNi44OTYsNjEsMTgsNjF6Ii8+PHBhdGggZD0iTTIxLDY0Yy0xLjEwNCwwLTIsMC44OTYtMiwyczAuODk2LDIsMiwyaDIxYzEuMTA0LDAsMi0wLjg5NiwyLTJzLTAuODk2LTItMi0ySDIxeiIvPjxwYXRoIGQ9Ik00OCw3MUgxOGMtMS4xMDQsMC0yLDAuODk2LTIsMnMwLjg5NiwyLDIsMmgzMGMxLjEwNCwwLDItMC44OTYsMi0yUzQ5LjEwNCw3MSw0OCw3MXoiLz48L2c+PC9nPjwvc3ZnPg==" width=(width); }
            }
            WeatherCode::DrizzleLight => todo!(),
            WeatherCode::DrizzleModerate => todo!(),
            WeatherCode::DrizzleDense => todo!(),
            WeatherCode::RainSlight | WeatherCode::RainModerate | WeatherCode::RainHeavy => {
                html! { img src="data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiA/PjwhRE9DVFlQRSBzdmcgIFBVQkxJQyAnLS8vVzNDLy9EVEQgU1ZHIDEuMS8vRU4nICAnaHR0cDovL3d3dy53My5vcmcvR3JhcGhpY3MvU1ZHLzEuMS9EVEQvc3ZnMTEuZHRkJz48c3ZnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXcgMCAwIDk2IDk2IiBoZWlnaHQ9Ijk2cHgiIHZlcnNpb249IjEuMSIgdmlld0JveD0iMCAwIDk2IDk2IiB3aWR0aD0iOTZweCIgeG1sOnNwYWNlPSJwcmVzZXJ2ZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIiB4bWxuczp4bGluaz0iaHR0cDovL3d3dy53My5vcmcvMTk5OS94bGluayI+PGcgaWQ9IkRpYnVqbyI+PGc+PHBhdGggZD0iTTY2LDQwYy0wLjUwNywwLTEuMTEyLDAuMDc5LTEuNjg4LDAuMTg0QzYyLjIxNywzMy4wMTIsNTUuNjYzLDI4LDQ4LDI4cy0xNC4yMTgsNS4wMTItMTYuMzExLDEyLjE4NCAgICBDMzEuMTEyLDQwLjA3OSwzMC41MDcsNDAsMzAsNDBjLTYuMDY1LDAtMTEsNC45MzUtMTEsMTFzNC45MzUsMTEsMTEsMTFjMS4xMDQsMCwyLTAuODk2LDItMnMtMC44OTYtMi0yLTJjLTMuODYsMC03LTMuMTQxLTctNyAgICBzMy4xNC03LDctN2MwLjI3NywwLDAuNzIzLDAuMDY4LDEuMTkzLDAuMTYyVjQ2YzAsMS4xMDQsMC44OTYsMiwyLDJzMi0wLjg5NiwyLTJ2LTMuMjIxQzM2LjI2NywzNi41MjcsNDEuNjMsMzIsNDgsMzIgICAgczExLjczMiw0LjUyNywxMi44MDcsMTAuNzc5VjQ2YzAsMS4xMDQsMC44OTYsMiwyLDJzMi0wLjg5NiwyLTJ2LTEuODM4QzY1LjI3Nyw0NC4wNjgsNjUuNzIyLDQ0LDY2LDQ0YzMuODU5LDAsNywzLjE0MSw3LDcgICAgcy0zLjE0MSw3LTcsN2MtMS4xMDQsMC0yLDAuODk2LTIsMnMwLjg5NiwyLDIsMmM2LjA2NSwwLDExLTQuOTM1LDExLTExUzcyLjA2NSw0MCw2Niw0MHoiLz48cGF0aCBkPSJNNDkuNDg1LDUyLjA2Yy0xLjA3My0wLjI3LTIuMTU4LDAuMzg0LTIuNDI2LDEuNDU1bC02LDI0Yy0wLjI2OCwxLjA3MiwwLjM4NCwyLjE1NywxLjQ1NSwyLjQyNiAgICBDNDIuNjc3LDc5Ljk4MSw0Mi44NCw4MCw0My4wMDEsODBjMC44OTYsMCwxLjcxMS0wLjYwNiwxLjkzOS0xLjUxNWw2LTI0QzUxLjIwOCw1My40MTMsNTAuNTU3LDUyLjMyOCw0OS40ODUsNTIuMDZ6Ii8+PHBhdGggZD0iTTU3LjQ4NCw1OC4wNmMtMS4wNzItMC4yNzEtMi4xNTcsMC4zODQtMi40MjUsMS40NTVsLTMsMTJjLTAuMjY4LDEuMDcyLDAuMzg0LDIuMTU4LDEuNDU2LDIuNDI2ICAgIGMwLjE2MywwLjA0MSwwLjMyNiwwLjA2LDAuNDg2LDAuMDZjMC44OTYsMCwxLjcxMi0wLjYwNiwxLjkzOS0xLjUxNWwyLjk5OS0xMkM1OS4yMDgsNTkuNDEzLDU4LjU1Niw1OC4zMjcsNTcuNDg0LDU4LjA2eiIvPjxwYXRoIGQ9Ik0zOC40ODQsNTguMDZjLTEuMDY5LTAuMjcxLTIuMTU3LDAuMzg0LTIuNDI1LDEuNDU1bC0zLDEyYy0wLjI2OCwxLjA3MiwwLjM4NCwyLjE1OCwxLjQ1NiwyLjQyNiAgICBjMC4xNjMsMC4wNDEsMC4zMjYsMC4wNiwwLjQ4NiwwLjA2YzAuODk2LDAsMS43MTItMC42MDYsMS45MzktMS41MTVsMy0xMkM0MC4yMDgsNTkuNDEzLDM5LjU1Niw1OC4zMjcsMzguNDg0LDU4LjA2eiIvPjwvZz48L2c+PC9zdmc+" width=(width); }
            }
            WeatherCode::Thunderstorm => todo!(),
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
