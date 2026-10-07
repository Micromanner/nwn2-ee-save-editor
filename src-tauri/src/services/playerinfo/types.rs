#[derive(Debug, PartialEq)]
pub struct PlayerClassEntry {
    pub name: String,
    pub level: u8,
}

impl PlayerClassEntry {
    pub fn new(name: impl Into<String>, level: u8) -> Self {
        Self {
            name: name.into(),
            level,
        }
    }
}

/// Layout per Arbos' playerinfo.bin reverse-engineering:
/// <https://gist.github.com/Arbos/225c724f91309d3f515e0f110524feee>
/// Verified against all 7 save fixtures in `tests/fixtures/saves/`.
#[derive(Debug, Default)]
pub struct PlayerInfoData {
    pub first_name: String,
    /// Always stored; a character without a last name has an empty string.
    pub last_name: String,

    pub subrace: String,
    pub alignment: String,

    /// Good/Evil axis: 4 = Good, 1 = Neutral, 5 = Evil.
    pub alignment_vertical: u32,
    /// Law/Chaos axis: 2 = Lawful, 1 = Neutral, 3 = Chaotic.
    pub alignment_horizontal: u32,
    /// Row id in `backgrounds.2da`: drives the background shown in the load menu.
    pub background_id: u32,

    pub classes: Vec<PlayerClassEntry>,

    pub deity: String,

    /// Str, Dex, Con, Int, Wis, Cha.
    pub scores: [u32; 6],
    /// Modifiers of `scores`, same order.
    pub modifiers: [i32; 6],
}

impl PlayerInfoData {
    pub fn full_name(&self) -> String {
        if self.last_name.is_empty() {
            self.first_name.clone()
        } else {
            format!("{} {}", self.first_name, self.last_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_name() {
        let mut data = PlayerInfoData {
            first_name: "John".to_string(),
            ..Default::default()
        };
        assert_eq!(data.full_name(), "John");
        data.last_name = "Doe".to_string();
        assert_eq!(data.full_name(), "John Doe");
    }
}
