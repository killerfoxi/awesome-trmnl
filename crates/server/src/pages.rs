use std::borrow::Borrow;

use maud::{DOCTYPE, Markup, html};

pub fn index(inner: impl Borrow<Markup>) -> Markup {
    html! {
        (DOCTYPE)
        head {
            title { "Awesome TRMNL" }
            link rel="stylesheet" href="/assets/style.css";
        }
        body {
            (inner.borrow())
        }
    }
}

pub fn error(title: &str, details: &str) -> Markup {
    index(html! {
        h1 { (title) }
        p { (details) }
    })
}

pub fn not_found(details: &str) -> Markup {
    error("A 404 has been spotted", details)
}

pub fn bad_request(details: &str) -> Markup {
    error("You tried a naughty thing", details)
}

pub fn internal_error(details: &str) -> Markup {
    error("I'm terribly sorry, but something happened", details)
}

pub fn test_screen() -> Markup {
    html! {
        div ."view view--full" {
            div .layout {
                div .columns {
                    div .column {
                        div .markdown {
                            span .title { "Motivational Quote" }
                            div ."content content--center" {
                                r#"“I love inside jokes. I hope to be a part of one
                                    someday.”"#
                            }
                            span ."label label--underline" { "Michael Scott" }
                        }
                    }
                }
            }

            div .title_bar {
                img .image src="https://usetrmnl.com/images/plugins/trmnl--render.svg";
                span .title { "Plugin Title" }
                span .instance { "Instance Title" }
            }
        }
    }
}

pub fn screen(inner: impl Borrow<Markup>) -> Markup {
    html! {
        (DOCTYPE)
        head {
            title { "Awesome TRMNL" }
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            link rel="stylesheet" href="https://usetrmnl.com/css/latest/plugins.css";
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200";
            link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;350;375;400;450;600;700&display=swap" rel="stylesheet";
        }
        body .environment.trmnl {
            div .screen {
                (inner.borrow())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_produces_markup() {
        let markup = index(&html! { p { "Hello" } });
        let s = markup.into_string();
        assert!(s.contains("<!DOCTYPE html>"));
        assert!(s.contains("Hello"));
        assert!(s.contains("style.css"));
    }

    #[test]
    fn error_produces_markup() {
        let markup = error("Oops", "Something went wrong");
        let s = markup.into_string();
        assert!(s.contains("Oops"));
        assert!(s.contains("Something went wrong"));
    }

    #[test]
    fn not_found_produces_markup() {
        let markup = not_found("Missing");
        let s = markup.into_string();
        assert!(s.contains("404"));
        assert!(s.contains("Missing"));
    }

    #[test]
    fn bad_request_produces_markup() {
        let markup = bad_request("Bad");
        let s = markup.into_string();
        assert!(s.contains("naughty thing"));
        assert!(s.contains("Bad"));
    }

    #[test]
    fn internal_error_produces_markup() {
        let markup = internal_error("Boom");
        let s = markup.into_string();
        assert!(s.contains("terribly sorry"));
        assert!(s.contains("Boom"));
    }

    #[test]
    fn test_screen_produces_markup() {
        let markup = test_screen();
        let s = markup.into_string();
        assert!(s.contains("Motivational Quote"));
        assert!(s.contains("Michael Scott"));
    }

    #[test]
    fn screen_produces_markup() {
        let inner = html! { p { "Content" } };
        let markup = screen(&inner);
        let s = markup.into_string();
        assert!(s.contains("<!DOCTYPE html>"));
        assert!(s.contains("Content"));
        assert!(s.contains("plugins.css"));
    }
}
