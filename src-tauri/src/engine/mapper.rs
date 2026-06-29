use crate::config::types::{InputSource, MappingRule};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

lazy_static::lazy_static! {
    pub static ref ENGINE: MappingEngine = MappingEngine::new();
}

pub struct MappingEngine {
    rules: Arc<RwLock<Vec<MappingRule>>>,
    exempt_queue: Arc<RwLock<HashMap<u32, std::time::Instant>>>,
}

impl MappingEngine {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            exempt_queue: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn load_rules(&self, rules: Vec<MappingRule>) {
        let mut r = self.rules.write();
        *r = rules;
        info!("Loaded {} mapping rules", r.len());
    }

    /// Find matching rules. `gamepad_buttons` is the current bitmask of pressed gamepad buttons (W3C indices).
    pub fn find_matching_rules(&self, source: &InputSource, gamepad_buttons: u32) -> Vec<MappingRule> {
        let rules = self.rules.read();
        let mut matches: Vec<MappingRule> = rules
            .iter()
            .filter(|r| r.is_enabled && self.source_matches(&r.source, source, gamepad_buttons))
            .cloned()
            .collect();

        // Sort by priority (lower number = higher priority)
        matches.sort_by_key(|r| r.priority);
        matches
    }

    fn source_matches(&self, rule_source: &InputSource, event_source: &InputSource, gamepad_buttons: u32) -> bool {
        // Check device type
        if rule_source.device != crate::config::types::DeviceType::Any
            && rule_source.device != event_source.device
        {
            return false;
        }

        // Check primary key
        if rule_source.primary_key != event_source.primary_key {
            return false;
        }

        // Check modifiers (all required modifiers must be pressed)
        for modifier in &rule_source.modifiers {
            if !event_source.modifiers.contains(modifier) {
                return false;
            }
        }

        // Check combo keys (all combo keys must be pressed simultaneously)
        if !rule_source.combo_keys.is_empty() {
            for combo_key in &rule_source.combo_keys {
                if gamepad_buttons & (1 << combo_key) == 0 {
                    return false;
                }
            }
        }

        // Check trigger mode
        if rule_source.mode != event_source.mode {
            return false;
        }

        true
    }

    pub fn add_exempt(&self, key: u32) {
        let mut queue = self.exempt_queue.write();
        queue.insert(key, std::time::Instant::now());
    }

    pub fn is_exempt(&self, key: u32) -> bool {
        let mut queue = self.exempt_queue.write();
        // Clean old entries (>100ms)
        queue.retain(|_, time| time.elapsed().as_millis() < 100);
        queue.contains_key(&key)
    }
}
