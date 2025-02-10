use chrono::Duration;
use maud::{html, Markup};

pub fn content() -> Markup {
    html! {
        div ."view view--full" {
            div ."layout layout--col layout--stretch-x" {
                (status_bar())
                div ."border--h-1" {}
                div .stretch {
                    (todos())
                }
            }
        }
    }
}

fn todos() -> Markup {
    html! {
        div ."flex flex--left flex--col" {
            (entry("Todo 1", None))
            (entry("Todo 2", Some(Duration::days(2))))
            (entry("Todo 3", Some(Duration::days(-3))))
            (entry("Todo 4", Some(Duration::days(0))))
        }
    }
}

fn entry(content: &str, deadline: Option<chrono::Duration>) -> Markup {
    use std::cmp::Ordering;

    let deadline = deadline.map(|dl| match dl.num_days().cmp(&0) {
        Ordering::Less => format!("{}d ago", dl.num_days().abs()),
        Ordering::Equal => "today".into(),
        Ordering::Greater => format!("in {}d", dl.num_days()),
    });
    html! {
        div .item {
            div .meta {}
            div .content {
                span ."title title--small" { (content) }
                div ."flex" {
                    @if let Some(deadline) = deadline {
                        span ."label label--small label--inverted" {
                            (deadline)
                        }
                    }
                }
            }
        }
    }
}

fn text_with_icon(icon: &str, text: &str) -> Markup {
    html! {
        div ."flex flex--row gap--small" {
            span .{"iconoir-" (icon)} {}
            span .label { (text) }
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
