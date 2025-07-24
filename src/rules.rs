use serde::{Deserialize, Serialize};
use std::path::Path;

/// Основной интерфейс классификатора
pub trait RuleEngine: Send + Sync {
    fn classify(&self, file: &Path) -> String;
}

/* ------------------------------------------------------------------ */
/* 1. Простейший классификатор — по расширению                         */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionRuleEngine;

impl RuleEngine for ExtensionRuleEngine {
    fn classify(&self, file: &Path) -> String {
        file.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_else(|| "no_extension".to_string())
    }
}

/* ------------------------------------------------------------------ */
/* 2. Расширяемые пользовательские правила (загружаются из JSON)       */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    /// Например, "jpg|jpeg|png" → "Images"
    pub pattern: String,
    pub target_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRuleEngine {
    pub rules: Vec<CustomRule>,
    pub fallback: String,
}

impl RuleEngine for CustomRuleEngine {
    fn classify(&self, file: &Path) -> String {
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        for rule in &self.rules {
            for token in rule.pattern.split('|') {
                if token.trim().eq_ignore_ascii_case(&ext) {
                    return rule.target_dir.clone();
                }
            }
        }
        self.fallback.clone()
    }
}

/* ------------------------------------------------------------------ */
/* 3. Blanket‑impl, чтобы Box<T> и Arc<T> тоже удовлетворяли RuleEngine*/
/* ------------------------------------------------------------------ */

use std::sync::Arc;

impl<T: RuleEngine + ?Sized> RuleEngine for Box<T> {
    fn classify(&self, file: &Path) -> String {
        (**self).classify(file)
    }
}

impl<T: RuleEngine + ?Sized> RuleEngine for Arc<T> {
    fn classify(&self, file: &Path) -> String {
        (**self).classify(file)
    }
}
