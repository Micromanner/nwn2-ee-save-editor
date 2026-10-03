use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct CharacterUpdates {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub age: Option<i32>,
    pub gender: Option<i32>,
    pub deity: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub alignment: Option<(i32, i32)>,
    pub experience: Option<i32>,
}
