use super::*;
use warp::test::request;
use std::net::{IpAddr, Ipv4Addr};

#[tokio::test]
async fn test_health_check() {
    let api = routes::routes();

    let response = request()
        .method("GET")
        .path("/health")
        .reply(&api)
        .await;

    assert_eq!(response.status(), 200);
    
    let body: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["timestamp"].is_string());
}

#[test]
fn test_backend_config() {
    // Test default config
    let config = BackendConfig::default();
    assert_eq!(config.host, IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_eq!(config.port, 3000);
    assert!(matches!(config.environment, Environment::Development));
    assert_eq!(config.cors_origins, vec!["http://localhost:3000"]);
    assert_eq!(config.request_timeout, 30);

    // Test environment loading
    std::env::set_var("BACKEND_PORT", "8080");
    std::env::set_var("BACKEND_ENV", "production");
    std::env::set_var("CORS_ORIGINS", "http://localhost:8080,http://example.com");
    std::env::set_var("REQUEST_TIMEOUT", "60");

    let config = BackendConfig::from_env();
    assert_eq!(config.port, 8080);
    assert!(matches!(config.environment, Environment::Production));
    assert_eq!(config.cors_origins, vec!["http://localhost:8080", "http://example.com"]);
    assert_eq!(config.request_timeout, 60);
}

#[test]
fn test_environment_helpers() {
    let mut config = BackendConfig::default();
    
    config.environment = Environment::Development;
    assert!(config.is_development());
    assert!(!config.is_production());

    config.environment = Environment::Production;
    assert!(!config.is_development());
    assert!(config.is_production());
}

#[test]
fn test_socket_addr() {
    let config = BackendConfig {
        host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        port: 8080,
        ..Default::default()
    };

    let addr = config.socket_addr();
    assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    assert_eq!(addr.port(), 8080);
} 