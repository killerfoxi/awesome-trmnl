use axum::response::Html;
use sailfish::TemplateOnce;

#[derive(TemplateOnce)]
#[template(path = "pages/index.stpl")]
struct IndexTemplate<'a> {
    inner: &'a str,
}

#[derive(TemplateOnce)]
#[template(path = "pages/screen.stpl")]
struct ScreenTemplate<'a> {
    inner: &'a str,
}

#[derive(TemplateOnce)]
#[template(path = "pages/error.stpl")]
struct ErrorTemplate<'a> {
    title: &'a str,
    details: &'a str,
}

#[derive(TemplateOnce)]
#[template(path = "pages/test_screen.stpl")]
struct TestScreenTemplate;

pub fn index(inner: &str) -> Html<String> {
    Html(
        IndexTemplate { inner }
            .render_once()
            .expect("index template render failed"),
    )
}

pub fn screen(inner: &str) -> Html<String> {
    Html(
        ScreenTemplate { inner }
            .render_once()
            .expect("screen template render failed"),
    )
}

pub fn error(title: &str, details: &str) -> Html<String> {
    Html(
        ErrorTemplate { title, details }
            .render_once()
            .expect("error template render failed"),
    )
}

pub fn not_found(details: &str) -> Html<String> {
    error("A 404 has been spotted", details)
}

pub fn bad_request(details: &str) -> Html<String> {
    error("You tried a naughty thing", details)
}

pub fn internal_error(details: &str) -> Html<String> {
    error("I'm terribly sorry, but something happened", details)
}

pub fn test_screen() -> String {
    TestScreenTemplate
        .render_once()
        .expect("test_screen template render failed")
}

pub fn home() -> Html<String> {
    index(concat!(
        "<h1>Welcome to Awesome TRMNL.</h1>",
        "<p>Do you have a TRMNL device? Point it at me.</p>",
        "<p>Or see a <a href=\"/preview/test\">test preview</a>.</p>",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_produces_markup() {
        let html = index("<p>Hello</p>").0;
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("style.css"));
    }

    #[test]
    fn error_produces_markup() {
        let html = error("Oops", "Something went wrong").0;
        assert!(html.contains("Oops"));
        assert!(html.contains("Something went wrong"));
    }

    #[test]
    fn not_found_produces_markup() {
        let html = not_found("Missing").0;
        assert!(html.contains("404"));
        assert!(html.contains("Missing"));
    }

    #[test]
    fn bad_request_produces_markup() {
        let html = bad_request("Bad").0;
        assert!(html.contains("naughty thing"));
        assert!(html.contains("Bad"));
    }

    #[test]
    fn internal_error_produces_markup() {
        let html = internal_error("Boom").0;
        assert!(html.contains("terribly sorry"));
        assert!(html.contains("Boom"));
    }

    #[test]
    fn test_screen_produces_markup() {
        let html = test_screen();
        assert!(html.contains("Motivational Quote"));
        assert!(html.contains("Michael Scott"));
    }

    #[test]
    fn screen_produces_markup() {
        let html = screen("<p>Content</p>").0;
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Content"));
        assert!(html.contains("plugins.css"));
    }
}
