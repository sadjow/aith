use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct EnvProfileSpec {
    from_env: Vec<EnvProfileMapping>,
    literals: Vec<EnvProfileMapping>,
}

#[derive(Debug)]
pub struct EnvProfileMapping {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnvProfileFile {
    env: BTreeMap<String, EnvValue>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum EnvValue {
    FromEnv { from_env: String },
    Literal(String),
}

impl EnvProfileSpec {
    pub fn new(
        from_env: Vec<EnvProfileMapping>,
        literals: Vec<EnvProfileMapping>,
    ) -> EnvProfileSpec {
        Self { from_env, literals }
    }
}

impl EnvProfileMapping {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

impl EnvProfileFile {
    pub fn from_spec(spec: EnvProfileSpec) -> Result<Self> {
        if spec.from_env.is_empty() && spec.literals.is_empty() {
            bail!("env profile must contain at least one env mapping");
        }

        let mut seen = BTreeSet::new();
        let mut env = BTreeMap::new();

        for mapping in spec.from_env {
            validate_env_name(&mapping.name, "target env name")?;
            validate_env_name(&mapping.value, "source env name")?;
            insert_unique(&mut seen, mapping.name.clone())?;
            env.insert(
                mapping.name,
                EnvValue::FromEnv {
                    from_env: mapping.value,
                },
            );
        }

        for mapping in spec.literals {
            validate_env_name(&mapping.name, "target env name")?;
            if is_sensitive_env_name(&mapping.name) {
                bail!(
                    "refusing to store literal value for sensitive env {}; use --from-env {}=SOURCE_ENV instead",
                    mapping.name,
                    mapping.name
                );
            }
            insert_unique(&mut seen, mapping.name.clone())?;
            env.insert(mapping.name, EnvValue::Literal(mapping.value));
        }

        Ok(Self { env })
    }

    pub fn read(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read env profile {}", path.display()))?;
        toml::from_str(&contents)
            .with_context(|| format!("failed to parse env profile {}", path.display()))
    }

    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).context("failed to serialize env profile")
    }

    pub fn resolve_env(&self) -> Result<Vec<(String, OsString)>> {
        let mut resolved = Vec::with_capacity(self.env.len());

        for (name, value) in &self.env {
            match value {
                EnvValue::FromEnv { from_env } => {
                    let Some(value) = std::env::var_os(from_env) else {
                        bail!("source env {} is not set for target env {}", from_env, name);
                    };
                    resolved.push((name.clone(), value));
                }
                EnvValue::Literal(value) => {
                    resolved.push((name.clone(), OsString::from(value)));
                }
            }
        }

        Ok(resolved)
    }
}

fn insert_unique(seen: &mut BTreeSet<String>, name: String) -> Result<()> {
    if seen.insert(name.clone()) {
        Ok(())
    } else {
        bail!("env target {} is configured more than once", name);
    }
}

fn validate_env_name(name: &str, label: &str) -> Result<()> {
    if name.is_empty() {
        bail!("{label} cannot be empty");
    }

    if name.len() > 128 {
        bail!("{label} cannot be longer than 128 characters");
    }

    let mut bytes = name.bytes();
    let first = bytes.next().expect("non-empty name");
    if !(first.is_ascii_alphabetic() || first == b'_') {
        bail!("{label} must start with an ASCII letter or '_'");
    }

    if !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
        bail!("{label} may only contain ASCII letters, numbers, and '_'");
    }

    Ok(())
}

fn is_sensitive_env_name(name: &str) -> bool {
    name.contains("API_KEY")
        || name.contains("AUTH_TOKEN")
        || name.contains("ACCESS_TOKEN")
        || name.contains("SECRET")
        || name.contains("PASSWORD")
}

#[cfg(test)]
mod tests {
    use super::{EnvProfileFile, EnvProfileMapping, EnvProfileSpec};

    #[test]
    fn serializes_source_env_references_without_secret_values() {
        let profile = EnvProfileFile::from_spec(EnvProfileSpec::new(
            vec![EnvProfileMapping::new(
                "ANTHROPIC_API_KEY",
                "ANTHROPIC_API_KEY_WORK",
            )],
            vec![EnvProfileMapping::new(
                "ANTHROPIC_BASE_URL",
                "https://example.test",
            )],
        ))
        .expect("profile");

        let serialized = profile.to_toml().expect("serialize");
        assert!(serialized.contains("from_env = \"ANTHROPIC_API_KEY_WORK\""));
        assert!(serialized.contains("ANTHROPIC_BASE_URL = \"https://example.test\""));
    }

    #[test]
    fn rejects_literal_sensitive_values() {
        let result = EnvProfileFile::from_spec(EnvProfileSpec::new(
            Vec::new(),
            vec![EnvProfileMapping::new("ANTHROPIC_API_KEY", "secret")],
        ));

        assert!(result.is_err());
    }
}
