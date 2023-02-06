#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use custom_uri::handle_custom_uri;
use http::Request;
use tauri::{async_runtime::block_on, http::ResponseBuilder};
use tokio::task::block_in_place;

mod custom_uri;

fn main() {
    tauri::Builder::default()
        // .invoke_handler(tauri::generate_handler![])
        .register_uri_scheme_protocol("spacedrive", move |_, req| {
            let uri = req.uri();
            let uri = uri
                .replace("spacedrive://localhost/", "http://spacedrive.localhost/") // Windows
                .replace("spacedrive://", "http://spacedrive.localhost/"); // Unix style

            // Encoded by `convertFileSrc` on the frontend
            let uri = percent_encoding::percent_decode(uri.as_bytes())
                .decode_utf8_lossy()
                .to_string();

            let mut r = Request::builder().method(req.method()).uri(uri);
            for (key, value) in req.headers() {
                r = r.header(key, value);
            }
            let r = r.body(req.body().clone()).unwrap(); // TODO: This clone feels so unnecessary but Tauri pass `req` as a reference so we can get the owned value.

            // TODO: This blocking sucks but is required for now. https://github.com/tauri-apps/wry/issues/420
            let resp = block_in_place(|| block_on(handle_custom_uri(r)));
            let mut r = ResponseBuilder::new()
                .version(resp.version())
                .status(resp.status());

            for (key, value) in resp.headers() {
                r = r.header(key, value);
            }

            r.body(resp.into_body())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
