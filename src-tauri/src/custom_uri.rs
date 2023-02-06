use http::{Request, Response, StatusCode};
use http_range::HttpRange;
use std::{cmp::min, env, path::PathBuf};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, SeekFrom},
};

pub async fn handle_custom_uri(req: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = req
        .uri()
        .path()
        .strip_prefix('/')
        .unwrap_or_else(|| req.uri().path())
        .split('/')
        .collect::<Vec<_>>();
    match path.first().copied() {
        Some("file") => {
            let file_name = path.get(1).unwrap();

            let path = env::current_dir()
                .unwrap()
                .join("../assets")
                .join(file_name);
            println!("path: {:?}", path);

            let mut file = File::open(&path).await.unwrap();
            let metadata = file.metadata().await.unwrap();

            // TODO: This should be determined from magic bytes when the file is indexer and stored it in the DB on the file path
            let (mime_type, is_video) = match path.extension().unwrap_or_default().to_str() {
                Some("mp4") => ("video/mp4", true),
                Some("webm") => ("video/webm", true),
                Some("mkv") => ("video/x-matroska", true),
                Some("avi") => ("video/x-msvideo", true),
                Some("mov") => ("video/quicktime", true),
                Some("png") => ("image/png", false),
                Some("jpg") => ("image/jpeg", false),
                Some("jpeg") => ("image/jpeg", false),
                Some("gif") => ("image/gif", false),
                Some("webp") => ("image/webp", false),
                Some("svg") => ("image/svg+xml", false),
                _ => todo!(),
            };

            match is_video {
                true => {
                    let mut response = Response::builder();
                    let mut status_code = 200;

                    // if the webview sent a range header, we need to send a 206 in return
                    // Actually only macOS and Windows are supported. Linux will ALWAYS return empty headers.
                    let buf = match req.headers().get("range") {
                        Some(range) => {
                            let mut buf = Vec::new();
                            let file_size = metadata.len();
                            let range =
                                HttpRange::parse(range.to_str().unwrap(), file_size).unwrap();
                            // let support only 1 range for now
                            let first_range = range.first();
                            if let Some(range) = first_range {
                                let mut real_length = range.length;

                                // prevent max_length;
                                // specially on webview2
                                if range.length > file_size / 3 {
                                    // max size sent (400ko / request)
                                    // as it's local file system we can afford to read more often
                                    real_length = min(file_size - range.start, 1024 * 400);
                                }

                                // last byte we are reading, the length of the range include the last byte
                                // who should be skipped on the header
                                let last_byte = range.start + real_length - 1;
                                status_code = 206;

                                // Only macOS and Windows are supported, if you set headers in linux they are ignored
                                response = response
                                    .header("Connection", "Keep-Alive")
                                    .header("Accept-Ranges", "bytes")
                                    .header("Content-Length", real_length)
                                    .header(
                                        "Content-Range",
                                        format!(
                                            "bytes {}-{}/{}",
                                            range.start, last_byte, file_size
                                        ),
                                    );

                                // FIXME: Add ETag support (caching on the webview)

                                file.seek(SeekFrom::Start(range.start)).await.unwrap();
                                file.take(real_length).read_to_end(&mut buf).await.unwrap();
                            } else {
                                file.read_to_end(&mut buf).await.unwrap();
                            }

                            buf
                        }
                        None => {
                            // Linux is mega cringe and doesn't support streaming so we just load the whole file into memory and return it
                            let mut buf = Vec::with_capacity(metadata.len() as usize);
                            file.read_to_end(&mut buf).await.unwrap();
                            buf
                        }
                    };

                    response
                        .header("Content-type", mime_type)
                        .status(status_code)
                        .body(buf)
                        .unwrap()
                }
                false => {
                    let mut buf = Vec::with_capacity(metadata.len() as usize);
                    file.read_to_end(&mut buf).await.unwrap();
                    Response::builder()
                        .header("Content-Type", mime_type)
                        .status(StatusCode::OK)
                        .body(buf)
                        .unwrap()
                }
            }
        }
        _ => todo!("Invalid operation!"),
    }
}
