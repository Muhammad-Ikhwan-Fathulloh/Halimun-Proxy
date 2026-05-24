use std::sync::Arc;
use tokio::time::{self, Duration};
use crate::services::registry::ServiceRegistry;
use reqwest::Client;

pub async fn start_health_checker(registry: Arc<ServiceRegistry>) {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let mut interval = time::interval(Duration::from_secs(30));

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            
            let services = registry.all_services();
            for service in services {
                if service.health.is_empty() {
                    continue;
                }
                
                let health_url = format!("{}{}", service.url, service.health);
                let is_healthy = match client.get(&health_url).send().await {
                    Ok(resp) => resp.status().is_success(),
                    Err(_) => false,
                };

                // Update health map internal state
                // This is a minimal example
            }
        }
    });
}
