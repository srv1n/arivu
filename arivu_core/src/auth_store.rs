use crate::auth::AuthDetails;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store unavailable: {0}")]
    Unavailable(String),
    #[error("persist error: {0}")]
    Persist(String),
}

pub trait AuthStore: Send + Sync {
    fn load(&self, provider: &str) -> Option<AuthDetails>;
    fn save(&self, provider: &str, auth: &AuthDetails) -> Result<(), StoreError>;
}

/// A simple in-memory store, mainly for testing.
pub struct MemoryAuthStore {
    map: std::sync::Mutex<std::collections::HashMap<String, AuthDetails>>,
}

impl MemoryAuthStore {
    pub fn new() -> Self {
        Self {
            map: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MemoryAuthStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthStore for MemoryAuthStore {
    fn load(&self, provider: &str) -> Option<AuthDetails> {
        self.map.lock().ok()?.get(provider).cloned()
    }
    fn save(&self, provider: &str, auth: &AuthDetails) -> Result<(), StoreError> {
        self.map
            .lock()
            .map_err(|e| StoreError::Persist(format!("lock poisoned: {}", e)))?
            .insert(provider.to_string(), auth.clone());
        Ok(())
    }
}

/// A simple file-backed JSON store at `~/.config/rzn_datasourcer/auth.json` (Unix)
/// or `%APPDATA%/rzn_datasourcer/auth.json` (Windows).
pub struct FileAuthStore {
    path: std::path::PathBuf,
}

impl FileAuthStore {
    pub fn new_default() -> Self {
        let base = dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|p| p.join(".config")))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let dir = base.join("rzn_datasourcer");
        let path = dir.join("auth.json");
        std::fs::create_dir_all(&dir).ok();
        Self { path }
    }

    fn read_map(&self) -> std::collections::HashMap<String, AuthDetails> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => std::collections::HashMap::new(),
        }
    }
    fn write_map(
        &self,
        map: &std::collections::HashMap<String, AuthDetails>,
    ) -> Result<(), StoreError> {
        let s = serde_json::to_string_pretty(map)
            .map_err(|e| StoreError::Persist(format!("serde: {}", e)))?;
        std::fs::write(&self.path, &s).map_err(|e| StoreError::Persist(e.to_string()))?;

        // Set restrictive permissions on Unix (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&self.path, perms)
                .map_err(|e| StoreError::Persist(format!("chmod: {}", e)))?;
        }

        Ok(())
    }
}

impl AuthStore for FileAuthStore {
    fn load(&self, provider: &str) -> Option<AuthDetails> {
        let map = self.read_map();
        map.get(provider).cloned()
    }

    fn save(&self, provider: &str, auth: &AuthDetails) -> Result<(), StoreError> {
        let mut map = self.read_map();
        map.insert(provider.to_string(), auth.clone());
        self.write_map(&map)
    }
}
