use maud::{html, Markup, DOCTYPE};

pub fn index(inner: Markup) -> Markup {
    html! {
        (DOCTYPE)
        title { "Awesome TRMNL" }
        body {
            (inner)
        }
    }
}

pub fn not_found(details: Markup) -> Markup {
    index(html! {
        h1 { "A 404 has been spotted" }
        (details)
    })
}

pub fn bad_request(details: Markup) -> Markup {
    index(html! {
        h1 { "You tried a nughty thing" }
        (details)
    })
}

pub fn internal_error(details: Markup) -> Markup {
    index(html! {
        h1 { "I'm terribly sorry, but something happened" }
        (details)
    })
}

pub fn test_screen() -> Markup {
    screen(html! {
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
    })
}

pub fn screen(inner: Markup) -> Markup {
    html! {
        (DOCTYPE)
        title { "Awesome TRMNL" }
        link rel="stylesheet" href="https://usetrmnl.com/css/latest/plugins.css";
        body .environment.trmnl {
            div .screen {
                (inner)
            }
        }
    }
}
