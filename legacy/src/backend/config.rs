use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Backend server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Server host address
    pub host: IpAddr,
    /// Server port
    pub port: u16,
    /// Environment (development, production, etc.)
    pub environment: Environment,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Request timeout in seconds
    pub request_timeout: u64,
}

/// Server environment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 3000,
            environment: Environment::Development,
            cors_origins: vec!["http://localhost:3000".to_string()],
            request_timeout: 30,
        }
    }
}

impl BackendConfig {
    /// Get the socket address for the server
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.environment == Environment::Development
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(port) = std::env::var("BACKEND_PORT") {
            if let Ok(port) = port.parse() {
                config.port = port;
            }
        }

        if let Ok(env) = std::env::var("BACKEND_ENV") {
            config.environment = match env.to_lowercase().as_str() {
                "production" => Environment::Production,
                "test" => Environment::Test,
                _ => Environment::Development,
            };
        }

        if let Ok(origins) = std::env::var("CORS_ORIGINS") {
            config.cors_origins = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        if let Ok(timeout) = std::env::var("REQUEST_TIMEOUT") {
            if let Ok(timeout) = timeout.parse() {
                config.request_timeout = timeout;
            }
        }

        config
    }
} 