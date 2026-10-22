use super::*;

#[tokio::test]
async fn favicon_is_public_and_serves_the_existing_png() {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let response = request.get("/favicon.ico").await;
    let status = response.status_code();
    let content_type = response.headers().get("content-type").cloned();
    let image = response.as_bytes().to_vec();
    let login = request.get("/login").await;
    let login_html = login.text();
    drop(request);
    close(fixture, boot).await;

    assert_eq!(status, 200);
    assert_eq!(
        content_type.as_ref().and_then(|value| value.to_str().ok()),
        Some("image/png")
    );
    assert_eq!(image, include_bytes!("../../assets/static/icon.png"));
    assert!(login_html.contains("<link rel=\"icon\" type=\"image/png\" href=\"/favicon.ico\">"));
}
