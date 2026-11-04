//! Every HTTP client in the app is built here, so all requests share one user agent.

use std::time::Duration;

/// `idle` bounds connecting and each read, turning a stalled connection into an error instead of
/// a hang. `None` waits forever.
pub fn http_client(idle: Option<Duration>) -> reqwest::Result<reqwest::Client> {
    http_client_builder(idle).build()
}

/// [`http_client`] before `build`, for a client that needs extra settings.
pub fn http_client_builder(idle: Option<Duration>) -> reqwest::ClientBuilder {
    let builder = reqwest::Client::builder().user_agent(concat!("translator-overlay/", env!("CARGO_PKG_VERSION")));
    match idle {
        Some(idle) => builder.connect_timeout(idle).read_timeout(idle),
        None => builder,
    }
}
