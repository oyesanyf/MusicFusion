use axum::http::Uri;
use quorum_ui::{get_asset, get_index_html, static_handler};

#[test]
fn test_index_html_contains_chatgpt_aesthetics() {
    let html = get_index_html();
    assert!(!html.is_empty(), "index.html must not be empty");

    // 1. 260px sidebar requirement
    assert!(html.contains("260px"), "Must contain 260px sidebar definition");

    // 2. 5-theme switcher variables
    assert!(
        html.contains("theme-white")
            && html.contains("theme-obsidian")
            && html.contains("theme-midnight")
            && html.contains("theme-warm")
            && html.contains("theme-dark"),
        "Must contain all 5 themes"
    );

    // 3. 768px reading column requirement
    assert!(
        html.contains("max-width: 768px"),
        "Must contain centered 768px reading column"
    );

    // 4. Citation badge styling requirement
    assert!(
        html.contains("citation-badge"),
        "Must contain citation-badge elements"
    );

    // 5. Model selector pill bar
    assert!(html.contains("Auto-Fusion") && html.contains("GLM-4-9B") && html.contains("Qwen 2.5") && html.contains("Gemma 3"));

    // 6. 3-way search mode toggle
    assert!(html.contains("BM25 Keyword") && html.contains("Semantic Vector") && html.contains("Hybrid RRF"));
}

#[test]
fn test_embedded_assets_presence_and_mime() {
    // styles.css
    let (css_bytes, css_mime) = get_asset("styles.css").expect("styles.css must exist");
    assert!(!css_bytes.is_empty());
    assert_eq!(css_mime, "text/css");

    // app.js
    let (js_bytes, js_mime) = get_asset("app.js").expect("app.js must exist");
    assert!(!js_bytes.is_empty());
    assert!(js_mime.contains("javascript"));

    // langextract.html
    let (lx_bytes, lx_mime) = get_asset("langextract.html").expect("langextract.html must exist");
    assert!(!lx_bytes.is_empty());
    assert_eq!(lx_mime, "text/html");

    // favicon.svg
    let (svg_bytes, svg_mime) = get_asset("favicon.svg").expect("favicon.svg must exist");
    assert!(!svg_bytes.is_empty());
    assert_eq!(svg_mime, "image/svg+xml");
}

#[tokio::test]
async fn test_static_handler_routes() {
    // Root uri
    let resp = static_handler(Uri::from_static("/")).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    // Explicit asset
    let resp = static_handler(Uri::from_static("/styles.css")).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    // Asset with /assets/ prefix
    let resp = static_handler(Uri::from_static("/assets/styles.css")).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    assert_eq!(resp.headers().get("content-type").unwrap(), "text/css");

    let (css_bytes, css_mime) = get_asset("assets/styles.css").expect("assets/styles.css must resolve");
    assert_eq!(css_mime, "text/css");
    assert!(!css_bytes.is_empty());

    // SPA fallback
    let resp = static_handler(Uri::from_static("/chat/conversation/123")).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}
