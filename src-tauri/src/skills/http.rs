use std::sync::OnceLock;
use std::time::Duration;

use reqwest::Client;

pub fn client() -> Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .user_agent("napi-desktop-skills/0.1")
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new())
        })
        .clone()
}
