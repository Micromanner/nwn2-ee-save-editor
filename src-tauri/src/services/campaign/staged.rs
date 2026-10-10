//! Campaign and module variables set in the editor, held in memory until the save
//! is written.

use std::collections::BTreeMap;

use serde_json::Value;

use super::CampaignManager;
use super::content::ModuleVariables;
use crate::config::NWN2Paths;
use crate::parsers::xml::XmlData;
use crate::services::save_graph::editing::{VariableKey, VariableStore, variable_text};
use crate::services::savegame_handler::SaveGameHandler;
use crate::services::savegame_handler::backup_manifest::VariableChange;

/// Variables set in the Game State tabs; `VariableKey::module` `None` is
/// `globals.xml`. Reads overlay them, `QuestEdits::flush` writes them.
#[derive(Default)]
pub struct StagedVariables {
    values: BTreeMap<VariableKey, Value>,
}

impl StagedVariables {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&VariableKey, &Value)> {
        self.values.iter()
    }

    /// The staged value of `key`, module names compared case-insensitively.
    pub fn get(&self, key: &VariableKey) -> Option<&Value> {
        self.values
            .iter()
            .find(|(k, _)| {
                k.name == key.name
                    && k.value_type == key.value_type
                    && k.in_scope(key.module.as_deref())
            })
            .map(|(_, value)| value)
    }

    /// Whether anything is staged in `module` (globals for `None`).
    pub fn touches(&self, module: Option<&str>) -> bool {
        self.scope(module).next().is_some()
    }

    /// The modules with staged variables.
    pub fn modules(&self) -> impl Iterator<Item = &str> {
        self.values.keys().filter_map(|key| key.module.as_deref())
    }

    /// Stage `(name, value, type)` updates of `module` (`None` = globals); a value
    /// equal to the one on disk unstages its variable.
    pub fn stage(
        &mut self,
        handler: &SaveGameHandler,
        paths: &NWN2Paths,
        module: Option<&str>,
        updates: &[(String, String, String)],
    ) -> Result<(), String> {
        let updates = updates
            .iter()
            .map(|(name, value, kind)| {
                let key = VariableKey {
                    module: module.map(str::to_string),
                    name: name.clone(),
                    value_type: kind.clone(),
                };
                Ok((key, parse_value(kind, name, value)?))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let saved: Box<dyn VariableStore> = match module {
            None => Box::new(CampaignManager::get_campaign_variables(handler)?),
            Some(module) => {
                Box::new(CampaignManager::get_module_info_by_id(handler, paths, module)?.1)
            }
        };
        for (key, value) in updates {
            if saved.get(&key.value_type, &key.name).as_ref() == Some(&value) {
                self.values.remove(&key);
            } else {
                self.values.insert(key, value);
            }
        }
        Ok(())
    }

    /// Unstage the variables of `module` (globals for `None`).
    pub fn discard(&mut self, module: Option<&str>) {
        self.values.retain(|key, _| !key.in_scope(module));
    }

    /// `globals.xml` values with the staged ones in place.
    pub fn overlay_globals(&self, data: &mut XmlData) {
        self.overlay(None, data);
    }

    /// A module's variables with the staged ones in place.
    pub fn overlay_module(&self, module: &str, vars: &mut ModuleVariables) {
        self.overlay(Some(module), vars);
    }

    /// `(name, value, type)` updates of `module`, values as text.
    pub fn updates(&self, module: Option<&str>) -> Vec<(String, String, String)> {
        self.scope(module)
            .map(|(key, value)| {
                (
                    key.name.clone(),
                    variable_text(value),
                    key.value_type.clone(),
                )
            })
            .collect()
    }

    /// The staged values for the backup manifest.
    pub fn changes(&self) -> Vec<VariableChange> {
        self.values
            .iter()
            .map(|(key, value)| {
                VariableChange::new(key.module.clone(), &key.name, variable_text(value))
            })
            .collect()
    }

    fn overlay(&self, module: Option<&str>, store: &mut impl VariableStore) {
        for (key, value) in self.scope(module) {
            store.set(&key.value_type, key.name.clone(), value);
        }
    }

    fn scope<'a>(
        &'a self,
        module: Option<&'a str>,
    ) -> impl Iterator<Item = (&'a VariableKey, &'a Value)> {
        self.values
            .iter()
            .filter(move |(key, _)| key.in_scope(module))
    }
}

fn parse_value(kind: &str, name: &str, value: &str) -> Result<Value, String> {
    let invalid = |e: String| format!("Invalid {kind} value for '{name}': {e}");
    match kind {
        "int" | "bool" => value
            .parse::<i32>()
            .map(Value::from)
            .map_err(|e| invalid(e.to_string())),
        "float" => value
            .parse::<f32>()
            .map(Value::from)
            .map_err(|e| invalid(e.to_string())),
        "string" => Ok(Value::from(value)),
        _ => Err(format!("Unknown variable type: {kind}")),
    }
}
