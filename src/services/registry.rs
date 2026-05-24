use crate::config::{ServiceConfig, ServiceType};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ServiceRegistry {
    services: Arc<DashMap<String, ServiceConfig>>,
    health_status: Arc<DashMap<String, bool>>,
}

impl ServiceRegistry {
    pub fn new(configs: Vec<ServiceConfig>) -> Self {
        let services = Arc::new(DashMap::new());
        let health_status = Arc::new(DashMap::new());
        
        for config in configs {
            health_status.insert(config.name.clone(), true); // Assume healthy at start
            services.insert(config.name.clone(), config);
        }
        
        Self { services, health_status }
    }

    pub fn get_service(&self, name: &str) -> Option<ServiceConfig> {
        self.services.get(name).map(|v| v.clone())
    }

    pub fn is_healthy(&self, name: &str) -> bool {
        self.health_status.get(name).map(|v| *v).unwrap_or(false)
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
        let mut all = Vec::new();
        for item in self.services.iter() {
            all.push(item.value().clone());
        }
        all
    }
}
