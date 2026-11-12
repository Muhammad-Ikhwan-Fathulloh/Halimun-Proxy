use crate::config::ServiceConfig;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ServiceRegistry {
    services: Arc<DashMap<String, ServiceConfig>>,
    health_status: Arc<DashMap<String, bool>>,
}

impl ServiceRegistry {
    pub fn new(configs: Vec<ServiceConfig>) -> Self {
        let registry = Self {
            services: Arc::new(DashMap::new()),
            health_status: Arc::new(DashMap::new()),
        };
        for config in configs {
            registry.register_service(config);
        }
        registry
    }

    pub fn get_service(&self, name: &str) -> Option<ServiceConfig> {
        self.services.get(name).map(|v| v.clone())
    }

    /// Status terakhir dari health checker. Service yang belum pernah dicek dianggap sehat.
    pub fn is_healthy(&self, name: &str) -> bool {
        self.health_status.get(name).map(|v| *v).unwrap_or(true)
    }

    pub fn set_health(&self, name: &str, healthy: bool) {
        if self.services.contains_key(name) {
            self.health_status.insert(name.to_string(), healthy);
        }
    }

    pub fn register_service(&self, config: ServiceConfig) {
        self.health_status.insert(config.name.clone(), true);
        self.services.insert(config.name.clone(), config);
    }

    pub fn remove_service(&self, name: &str) {
        self.services.remove(name);
        self.health_status.remove(name);
    }

    pub fn all_services(&self) -> Vec<ServiceConfig> {
        let mut all: Vec<ServiceConfig> = self.services.iter().map(|e| e.value().clone()).collect();
        all.sort_by(|a, b| a.name.cmp(&b.name));
        all
    }
}
