//! Configuration for the Apollo client.

/// Configuration for the Apollo client.
///
/// This struct holds the necessary information to connect to an Apollo Configuration center.
pub struct Config {
    /// The unique identifier for your application.
    pub app_id: String,
    /// The cluster name to connect to (e.g., "default").
    pub cluster: String,
    /// List of Apollo server URLs to connect to.
    pub servers: Vec<String>,

    /// Secret key for authentication with the Apollo server.
    pub secret: Option<String>,
}
