use crate::services::registry::ServiceRegistry;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio::time;

const CHECK_INTERVAL: Duration = Duration::from_secs(30);
const CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// Menjalankan pengecekan kesehatan berkala. Semua service dicek PARALEL
/// (waktu siklus = max latensi, bukan jumlah latensi), dan hasilnya disimpan di registry.
pub fn start_health_checker(registry: Arc<ServiceRegistry>) {
    let client = Client::builder()
        .timeout(CHECK_TIMEOUT)
        .connect_timeout(CHECK_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("klien health check");

    tokio::spawn(async move {
        let mut interval = time::interval(CHECK_INTERVAL);
        loop {
            interval.tick().await;
            let mut set = JoinSet::new();
            for service in registry.all_services() {
                if service.health.is_empty() {
                    continue;
                }
                let client = client.clone();
                set.spawn(async move {
                    let url = format!("{}{}", service.url.trim_end_matches('/'), service.health);
                    let healthy =
                        matches!(client.get(&url).send().await, Ok(r) if r.status().is_success());
                    (service.name, healthy)
                });
            }
            while let Some(joined) = set.join_next().await {
                if let Ok((name, healthy)) = joined {
                    registry.set_health(&name, healthy);
                }
            }
        }
    });
}
