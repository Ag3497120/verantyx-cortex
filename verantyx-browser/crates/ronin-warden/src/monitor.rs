use sysinfo::System;
use tracing::{info, warn};

pub struct SystemMonitor {
    sys: System,
    memory_limit_mb: u64,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self {
            sys: System::new_all(),
            memory_limit_mb: 4096, // Default 4GB limit per agent process tree
        }
    }
}

impl SystemMonitor {
    pub fn new(memory_limit_mb: u64) -> Self {
        Self {
            sys: System::new_all(),
            memory_limit_mb,
        }
    }

    /// Refresh system information and check if any agent processes exceed resource thresholds
    pub fn check_health(&mut self) -> anyhow::Result<()> {
        self.sys.refresh_all();
        
        let total_mem = self.sys.total_memory() / 1024 / 1024;
        let used_mem = self.sys.used_memory() / 1024 / 1024;

        if used_mem > self.memory_limit_mb {
            warn!("[Warden] System memory usage exceeded threshold: {}MB / {}MB limit", used_mem, self.memory_limit_mb);
            // In a real scenario, Warden would identify top consuming processes and SIGTERM them.
        }

        Ok(())
    }
}
