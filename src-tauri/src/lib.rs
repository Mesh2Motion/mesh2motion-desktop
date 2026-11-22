use tauri::webview::WebviewWindowBuilder;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Tauri's embedded asset server guesses Content-Type from a short hardcoded
/// extension list (see tauri-utils' mime_type.rs) plus content sniffing via the
/// `infer` crate. Neither recognizes `.glb` (glTF binary) or the app's generated
/// `.mp4` animation previews, so both come back labelled `text/html` even though
/// the actual bytes served are correct. `fetch()`-based loaders (three.js'
/// GLTFLoader) ignore the header and work anyway, but `<video>` elements enforce
/// Content-Type strictly, which is why the animation library previews stayed
/// blank. Fix the header here before it reaches the webview.
///
/// The response also never carries a Content-Length, which makes WebKitGTK's
/// media pipeline reject the stream outright (MEDIA_ERR_SRC_NOT_SUPPORTED) even
/// once Content-Type is correct, since it can't tell the response is a complete,
/// playable resource. Set it from the body we're already holding.
fn fix_content_type(request: tauri::http::Request<Vec<u8>>, response: &mut tauri::http::Response<std::borrow::Cow<'static, [u8]>>) {
    let path = request.uri().path();
    let content_type = if path.ends_with(".glb") || path.ends_with(".gltf") {
        Some("model/gltf-binary")
    } else if path.ends_with(".mp4") {
        Some("video/mp4")
    } else {
        None
    };

    if let Some(content_type) = content_type {
        response.headers_mut().insert(
            tauri::http::header::CONTENT_TYPE,
            tauri::http::HeaderValue::from_static(content_type),
        );
        let len = response.body().len();
        response.headers_mut().insert(
            tauri::http::header::CONTENT_LENGTH,
            tauri::http::HeaderValue::from_str(&len.to_string()).unwrap(),
        );
        response.headers_mut().insert(
            tauri::http::header::ACCEPT_RANGES,
            tauri::http::HeaderValue::from_static("bytes"),
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK's DMA-BUF Wayland renderer produces flickering/garbage frames on some
    // Mesa/compositor combinations (same underlying class of bug that causes the
    // `Could not create default EGL display: EGL_BAD_PARAMETER` crash some AppImage
    // builds hit). Forcing the non-DMA-BUF compositing path fixes both. Linux-only,
    // and must be set before the webview is created.
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            let window_config = app
                .config()
                .app
                .windows
                .first()
                .cloned()
                .expect("expected at least one window in tauri.conf.json");

            WebviewWindowBuilder::from_config(app, &window_config)?
                .on_web_resource_request(fix_content_type)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
