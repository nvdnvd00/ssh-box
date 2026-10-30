// Tool Installer catalog and core types (spec §6). Pure: no I/O.
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HostOs {
    Linux,
    Macos,
    Windows,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Method {
    pub os: HostOs,
    pub manager: String,
    pub package: String,
    pub service: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogTool {
    pub id: String,
    pub name: String,
    pub category: String,
    pub recommended: bool,
    pub description_vi: String,
    pub description_en: String,
    pub bin: Option<String>,
    pub methods: Vec<Method>,
}

pub fn catalog() -> &'static [CatalogTool] {
    static CATALOG: OnceLock<Vec<CatalogTool>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("installer_catalog.json")).expect("installer_catalog.json")
    })
}

pub fn managers_for(os: HostOs) -> &'static [&'static str] {
    match os {
        HostOs::Linux => &["apt", "dnf", "snap", "npm"],
        HostOs::Macos => &["brew", "npm"],
        HostOs::Windows => &["winget", "npm"],
    }
}

pub fn pick_method<'a>(tool: &'a CatalogTool, os: HostOs, present: &[String]) -> Option<&'a Method> {
    tool.methods.iter().find(|m| m.os == os && present.contains(&m.manager))
}

// ^[A-Za-z0-9@/._+:-]{1,128}$ — the only names allowed into a generated script.
pub fn valid_name(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || "@/._+:-".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_ids_unique() {
        let c = catalog();
        assert!(!c.is_empty());
        let mut ids: Vec<&str> = c.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), c.len());
        for t in c {
            for m in &t.methods {
                assert!(managers_for(m.os).contains(&m.manager.as_str()), "{} {}", t.id, m.manager);
                assert!(valid_name(&m.package));
            }
        }
    }

    #[test]
    fn pick_method_prefers_first_present() {
        let t = catalog().iter().find(|t| t.id == "docker").unwrap();
        let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(pick_method(t, HostOs::Linux, &p(&["snap"])).unwrap().manager, "snap");
        assert_eq!(pick_method(t, HostOs::Linux, &p(&["apt", "snap"])).unwrap().manager, "apt");
        assert!(pick_method(t, HostOs::Linux, &[]).is_none());
    }

    #[test]
    fn valid_name_rejects_shell() {
        for ok in ["@anthropic-ai/claude-code", "Tailscale.Tailscale", "python3"] {
            assert!(valid_name(ok), "{ok}");
        }
        for bad in ["a;b", "a b", "$(x)", ""] {
            assert!(!valid_name(bad), "{bad}");
        }
        assert!(!valid_name(&"a".repeat(129)));
    }
}
