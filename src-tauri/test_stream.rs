use stream_download::{StreamDownload, Settings};
use stream_download::http::reqwest::Client;

fn main() {
    let client = reqwest::Client::new();
    let url = "http://example.com".parse().unwrap();
    // let stream = StreamDownload::new_http(url, client, Settings::default()).unwrap();
}
