use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to determine home directory")]
    NoHomeDir,

    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("{0} API key not configured. Please edit {1} and add your API key for {0}.")]
    MissingApiKey(String, String),

    #[error("Field not found: {0}")]
    FieldNotFound(String),
}

pub type ConfigResult<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    OptionalString,
    Enum,
    U64,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    String(String),
    OptionalString(Option<String>),
    Enum(String),
    U64(u64),
}

impl FieldValue {
    pub fn field_type(&self) -> FieldType {
        match self {
            FieldValue::String(_) => FieldType::String,
            FieldValue::OptionalString(_) => FieldType::OptionalString,
            FieldValue::Enum(_) => FieldType::Enum,
            FieldValue::U64(_) => FieldType::U64,
        }
    }

    pub fn as_display_string(&self) -> String {
        match self {
            FieldValue::String(s) => s.clone(),
            FieldValue::OptionalString(Some(s)) => s.clone(),
            FieldValue::OptionalString(None) => String::new(),
            FieldValue::Enum(s) => s.clone(),
            FieldValue::U64(n) => n.to_string(),
        }
    }

    pub fn from_string(s: String, field_type: FieldType) -> Self {
        let trimmed = s.trim().to_string();
        match field_type {
            FieldType::String => FieldValue::String(trimmed),
            FieldType::OptionalString => {
                if trimmed.is_empty() {
                    FieldValue::OptionalString(None)
                } else {
                    FieldValue::OptionalString(Some(trimmed))
                }
            }
            FieldType::Enum => FieldValue::Enum(trimmed),
            FieldType::U64 => FieldValue::U64(trimmed.parse().unwrap_or(0)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FieldDescriptor {
    pub id: String,
    pub label: String,
    pub hint: String,
    pub section: String,
    pub field_type: FieldType,
    pub enum_values: Vec<String>,
    pub is_group: bool,
    pub parent_id: Option<String>,
}

impl FieldDescriptor {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        hint: impl Into<String>,
        section: impl Into<String>,
        field_type: FieldType,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            hint: hint.into(),
            section: section.into(),
            field_type,
            enum_values: Vec::new(),
            is_group: false,
            parent_id: None,
        }
    }

    pub fn with_enum_values(mut self, values: Vec<String>) -> Self {
        self.enum_values = values;
        self
    }

    pub fn as_group(mut self) -> Self {
        self.is_group = true;
        self
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn get_value<C: ConfigSchema>(&self, config: &C) -> ConfigResult<FieldValue> {
        C::get_field_value_by_id(config, &self.id)
    }

    pub fn set_value<C: ConfigSchema>(
        &self,
        config: &mut C,
        value: FieldValue,
    ) -> ConfigResult<()> {
        C::set_field_value_by_id(config, &self.id, value)
    }
}

pub trait ConfigSchema: Sized + Serialize + for<'de> Deserialize<'de> + Default {
    fn fields() -> Vec<FieldDescriptor>;
    fn get_field_value_by_id(config: &Self, id: &str) -> ConfigResult<FieldValue>;
    fn set_field_value_by_id(config: &mut Self, id: &str, value: FieldValue) -> ConfigResult<()>;

    fn sections() -> Vec<String> {
        let mut sections = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for field in Self::fields() {
            if seen.insert(field.section.clone()) {
                sections.push(field.section.clone());
            }
        }

        sections
    }

    fn get_field(&self, field_id: &str) -> ConfigResult<FieldValue> {
        Self::fields()
            .iter()
            .find(|f| f.id == field_id)
            .ok_or_else(|| ConfigError::FieldNotFound(field_id.to_string()))?
            .get_value(self)
    }

    fn set_field(&mut self, field_id: &str, value: FieldValue) -> ConfigResult<()> {
        Self::fields()
            .iter()
            .find(|f| f.id == field_id)
            .ok_or_else(|| ConfigError::FieldNotFound(field_id.to_string()))?
            .set_value(self, value)
    }

    fn load() -> ConfigResult<Self> {
        Self::load_from(None)
    }

    fn load_from(custom_path: Option<PathBuf>) -> ConfigResult<Self> {
        let path = custom_path.clone().unwrap_or(Self::config_path()?);

        if !path.exists() {
            let config = Self::default();
            config.save_to(Some(path.clone()))?;
            eprintln!("Created default config at: {}", path.display());
            eprintln!("Please edit it to add your API keys.");
            return Ok(config);
        }

        let content = fs::read_to_string(&path)?;

        match serde_json::from_str::<Self>(&content) {
            Ok(config) => Ok(config),
            Err(e) => {
                eprintln!("Warning: Config file is corrupted or outdated.");
                eprintln!("Error: {}", e);

                let backup_path = Self::backup_path()?;
                fs::copy(&path, &backup_path)?;
                eprintln!("Backed up old config to: {}", backup_path.display());

                let config = Self::default();
                config.save_to(Some(path.clone()))?;
                eprintln!("Created new default config at: {}", path.display());

                Ok(config)
            }
        }
    }

    fn save(&self) -> ConfigResult<()> {
        self.save_to(None)
    }

    fn save_to(&self, custom_path: Option<PathBuf>) -> ConfigResult<()> {
        let path = custom_path.unwrap_or(Self::config_path()?);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        Ok(())
    }

    fn config_dir() -> ConfigResult<PathBuf> {
        let home = dirs::home_dir().ok_or(ConfigError::NoHomeDir)?;
        Ok(home.join(".onyx"))
    }

    fn config_path() -> ConfigResult<PathBuf> {
        Ok(Self::config_dir()?.join("config.json"))
    }

    fn backup_path() -> ConfigResult<PathBuf> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Ok(Self::config_dir()?.join(format!("config.json.backup.{}", timestamp)))
    }

    fn config_path_display() -> String {
        Self::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.onyx/config.json".to_string())
    }
}

#[macro_export]
macro_rules! config_defaults {
    ($($field:ident => $value:expr),* $(,)?) => {
        impl Default for Config {
            fn default() -> Self {
                Self {
                    $(
                        $field: $value.into(),
                    )*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! config_fields {
    {
        $(
            [$section:expr] => {
                $(
                    $id:ident: $ty:ident($($attrs:tt)*)
                ),* $(,)?
            }
        )*
    } => {
        impl $crate::config::ConfigSchema for Config {
            fn fields() -> Vec<$crate::config::FieldDescriptor> {
                vec![
                    $(
                        $(
                            config_fields!(@field $id, $ty, $section, $($attrs)*),
                        )*
                    )*
                ]
            }

            fn get_field_value_by_id(c: &Config, id: &str) -> $crate::config::ConfigResult<$crate::config::FieldValue> {
                Ok(match id {
                    $(
                        $(
                            stringify!($id) => config_fields!(@get $ty, c, $($attrs)*),
                        )*
                    )*
                    _ => return Err($crate::config::ConfigError::FieldNotFound(id.to_string())),
                })
            }

            fn set_field_value_by_id(c: &mut Config, id: &str, v: $crate::config::FieldValue) -> $crate::config::ConfigResult<()> {
                match id {
                    $(
                        $(
                            stringify!($id) => config_fields!(@set $ty, c, v, $($attrs)*),
                        )*
                    )*
                    _ => return Err($crate::config::ConfigError::FieldNotFound(id.to_string())),
                }
                Ok(())
            }
        }
    };

    (@field $id:ident, $ty:ident, $section:expr, $label:expr, $hint:expr, $($path:tt).+ $(, $enum_vals:expr)?) => {
        {
            #[allow(unused_mut)]
            let mut f = $crate::config::FieldDescriptor::new(stringify!($id), $label, $hint, $section, $crate::config::FieldType::$ty);
            $(f = f.with_enum_values($enum_vals);)?
            f
        }
    };

    (@get String, $c:expr, $label:expr, $hint:expr, $($path:tt).+ $(, $enum_vals:expr)?) => {
        $crate::config::FieldValue::String($c.$($path).+.clone())
    };
    (@get OptionalString, $c:expr, $label:expr, $hint:expr, $($path:tt).+ $(, $enum_vals:expr)?) => {
        $crate::config::FieldValue::OptionalString($c.$($path).+.clone())
    };
    (@get Enum, $c:expr, $label:expr, $hint:expr, $($path:tt).+, $enum_vals:expr) => {
        $crate::config::FieldValue::Enum($c.$($path).+.to_string())
    };
    (@get U64, $c:expr, $label:expr, $hint:expr, $($path:tt).+ $(, $enum_vals:expr)?) => {
        $crate::config::FieldValue::U64($c.$($path).+)
    };

    (@set String, $c:expr, $v:expr, $label:expr, $hint:expr, $($path:tt).+ $(, $enum_vals:expr)?) => {
        if let $crate::config::FieldValue::String(val) = $v {
            $c.$($path).+ = val;
        }
    };
    (@set OptionalString, $c:expr, $v:expr, $label:expr, $hint:expr, $($path:tt).+ $(, $enum_vals:expr)?) => {
        if let $crate::config::FieldValue::OptionalString(val) = $v {
            $c.$($path).+ = val;
        }
    };
    (@set Enum, $c:expr, $v:expr, $label:expr, $hint:expr, $($path:tt).+, $enum_vals:expr) => {
        if let $crate::config::FieldValue::Enum(val) = $v {
            $c.$($path).+ = val.parse().unwrap_or_default();
        }
    };
    (@set U64, $c:expr, $v:expr, $label:expr, $hint:expr, $($path:tt).+ $(, $enum_vals:expr)?) => {
        if let $crate::config::FieldValue::U64(val) = $v {
            $c.$($path).+ = val;
        }
    };
}
