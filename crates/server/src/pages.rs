use maud::{html, Markup, DOCTYPE};

pub fn index(inner: Markup) -> Markup {
    html! {
        (DOCTYPE)
        head {
            title { "Awesome TRMNL" }
            link rel="stylesheet" href="/assets/style.css";
        }
        body {
            (inner)
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

pub fn empty_screen() -> Markup {
    html! {
        div ."view view--full" {
            div .layout {
                div .columns {
                    div .column {
                        div .markdown {
                            span .title { "Nothing to render" }
                        }
                    }
                }
            }
        }
    }
}

pub fn screen(inner: Markup) -> Markup {
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
                (inner)
            }
        }
    }
}
