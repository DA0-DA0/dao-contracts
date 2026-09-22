use std::collections::HashMap;

use crate::{
    config::DeployInfo,
    error::{ProcessError, Result},
};

#[derive(Clone, Debug)]
pub struct ContractMap {
    entries: HashMap<String, DeployInfo>,
}

impl ContractMap {
    pub fn new(entries: HashMap<String, DeployInfo>) -> Self {
        Self { entries }
    }

    pub fn code_id(&self, name: &str) -> Result<u64> {
        self.entries
            .get(name)
            .and_then(|entry| entry.code_id)
            .ok_or_else(|| ProcessError::Registry(format!("{name} has not been stored")))
    }

    pub fn address(&self, name: &str) -> Result<String> {
        self.entries
            .get(name)
            .and_then(|entry| entry.address.clone())
            .ok_or_else(|| ProcessError::Registry(format!("{name} has not been instantiated")))
    }

    pub fn add_address(&mut self, name: &str, address: impl Into<String>) -> Result<()> {
        // Helpers register child-module addresses discovered by contract queries.
        self.entries.entry(name.into()).or_default().address = Some(address.into());
        Ok(())
    }

    pub(crate) fn register(&mut self, name: &str, code_id: u64) {
        self.entries.entry(name.into()).or_default().code_id = Some(code_id);
    }

    pub fn deploy_info(&self) -> &HashMap<String, DeployInfo> {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_code_address_and_named_variant_identity() {
        let mut registry = ContractMap::new(HashMap::new());
        assert!(registry.code_id("missing").is_err());
        assert!(registry.address("missing").is_err());
        registry.register("token-default", 1);
        registry.register("token-thorchain", 2);
        registry.add_address("token-default", "first").unwrap();
        registry.add_address("token-default", "second").unwrap();
        registry.register("token-default", 3);
        assert_eq!(registry.address("token-default").unwrap(), "second");
        assert_eq!(registry.code_id("token-default").unwrap(), 3);
        assert_eq!(registry.code_id("token-thorchain").unwrap(), 2);
        assert!(registry.code_id("token").is_err());
        let persisted = serde_yaml::to_string(registry.deploy_info()).unwrap();
        let restored = ContractMap::new(serde_yaml::from_str(&persisted).unwrap());
        assert_eq!(restored.deploy_info(), registry.deploy_info());
    }
}
