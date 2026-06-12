use super::server::{AcpServer, AcpServerStatus};
use std::collections::HashMap;
use std::sync::RwLock;

/// ACP Server 注册表。
pub struct AcpServerRegistry {
    servers: RwLock<HashMap<String, AcpServer>>,
}

impl AcpServerRegistry {
    pub fn new(servers: Vec<AcpServer>) -> Self {
        Self {
            servers: RwLock::new(
                servers
                    .into_iter()
                    .map(|server| (server.id.clone(), server))
                    .collect(),
            ),
        }
    }

    pub fn list(&self) -> Vec<AcpServer> {
        self.servers.read().unwrap().values().cloned().collect()
    }

    pub fn get(&self, id: &str) -> Option<AcpServer> {
        self.servers.read().unwrap().get(id).cloned()
    }

    pub fn save(&self, server: AcpServer) {
        self.servers
            .write()
            .unwrap()
            .insert(server.id.clone(), server);
    }

    pub fn status(&self, id: &str) -> AcpServerStatus {
        match self.get(id) {
            Some(server) if server.enabled => AcpServerStatus::Available,
            Some(_) => AcpServerStatus::Disabled,
            None => AcpServerStatus::Unknown,
        }
    }

    #[allow(dead_code)]
    pub fn remove(&self, id: &str) {
        self.servers.write().unwrap().remove(id);
    }
}
