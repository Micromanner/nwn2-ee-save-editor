pub const PRIORITY_TABLES: &[&str] = &[
    "classes",
    "racialtypes",
    "feat",
    "skills",
    "spells",
    "baseitems",
    "appearance",
    "gender",
    "alignment",
    "categories",
    "cls_atk_1",
    "cls_atk_2",
    "cls_atk_3",
    "backgrounds",
    "domains",
    "nwn2_icons",
];

pub const CHARACTER_PREFIXES: &[&str] = &[
    "actions",
    "ammunition",
    "appearance",
    "areaeffects",
    "armor",
    "backgrounds",
    "baseitems",
    "capart",
    "categories",
    "catype",
    "chargen",
    "classes",
    "cls_",
    "color_",
    "combatmodes",
    "creaturesize",
    "creaturespeed",
    "damage",
    "damagereductins",
    "des_feat2item",
    "des_matcomp",
    "des_pcstart_",
    "des_prayer",
    "des_restsystem",
    "des_xp_rewards",
    "disease",
    "domains",
    "effectanim",
    "effecticons",
    "encdifficulty",
    "encumbrance",
    "epic",
    "exptable",
    "feat",
    "gender",
    // Animal companion / familiar choices stored on the creature
    "hen_companion",
    "hen_familiar",
    "iprp_",
    "itemprop",
    "itemtypes",
    "itemvalue",
    "masterfeats",
    "metamagic",
    "nwn2_align",
    "nwn2_bloodtypes",
    "nwn2_colors",
    "nwn2_deities",
    "nwn2_icons",
    "parts_",
    "phenotype",
    "poison",
    "polymorph",
    "portraits",
    "race_",
    "racial",
    "ranges",
    "repadjust",
    "reput",
    "resistancecost",
    "rest",
    "skill_",
    "skills",
    "skillvsitemcost",
    "soundset",
    "spells",
    "spellschools",
    "spelltarget",
    "tailmodel",
    "tintmap",
    "traps",
    "treasurescale",
    "wingmodel",
    "xpbaseconst",
    "xptable",
];

pub const IGNORE_PREFIXES: &[&str] = &[
    "ambientmusic",
    "ambientsound",
    "appearancesndset",
    "atest",
    "bodybag",
    "container_preference",
    "crafting",
    "crft_",
    "crtemplates",
    "cursors",
    "defaultacsounds",
    "des_blumburg",
    "des_conf_treas",
    "des_crft_",
    "des_cutconvdur",
    "des_mechupgrades",
    "des_treas_",
    "diffsettings",
    "doortypes",
    "environ",
    "excitedduration",
    "footstepsounds",
    "fractionalcr",
    "ftext_styles",
    "game_params",
    "gamespy",
    "genericdoors",
    "grass",
    "henchspells",
    "inventorysnds",
    "itm_rand_",
    "itmwiz",
    "keymap",
    "light",
    "loadhints",
    "loadscreen",
    "metatiles",
    "namefilter",
    "nwconfig",
    "nwn2_animcom",
    "nwn2_animstan",
    "nwn2_behaviorparams",
    "nwn2_dmcommands",
    "nwn2_emotes",
    "nwn2_scriptsets",
    "nwn2_tips",
    "nwn2_voicemenu",
    "pack",
    "placeable",
    "pregen",
    "prioritygroups",
    "pvpsettings",
    "replacetexture",
    "rrf_",
    "screen_container_ui",
    "sky",
    "soundcatfilters",
    "sounddefaults",
    "soundeax",
    "soundgain",
    "soundprovider",
    "soundtypes",
    "statescripts",
    "stringtokens",
    "surfacemat",
    "swearfilter",
    "tcn01",
    "tdc01",
    "tde01",
    "tdm01",
    "tdr01",
    "tds01",
    "terrainmaterials",
    "texture",
    "tib01",
    "tic01",
    "tid01",
    "tii01",
    "tile",
    "time",
    "tin01",
    "tms01",
    "treas_",
    "trees",
    "ttr01",
    "tts01",
    "ttu01",
    "ttd01_edge",
    "ttf01_edge",
    "tti01_edge",
    "vfx_",
    "video",
    "visualeffects",
    "water",
    "waypoint",
    "weaponsounds",
];

pub fn is_character_table(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    CHARACTER_PREFIXES
        .iter()
        .any(|prefix| name_lower.starts_with(prefix))
}

pub fn is_ignored_table(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    IGNORE_PREFIXES
        .iter()
        .any(|prefix| name_lower.starts_with(prefix))
}

pub fn is_priority_table(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    PRIORITY_TABLES.iter().any(|&t| t == name_lower)
}

pub fn should_load_table(name: &str) -> bool {
    !is_ignored_table(name) && is_character_table(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_character_table() {
        assert!(is_character_table("classes.2da"));
        assert!(is_character_table("cls_feat_fighter.2da"));
        assert!(is_character_table("iprp_abilities.2da"));
        assert!(!is_character_table("ambientmusic.2da"));
    }

    #[test]
    fn test_pet_tables_load() {
        assert!(should_load_table("hen_companion.2da"));
        assert!(should_load_table("hen_familiar.2da"));
        assert!(!should_load_table("henchspells.2da"));
    }

    #[test]
    fn test_is_ignored_table() {
        assert!(is_ignored_table("ambientmusic.2da"));
        assert!(is_ignored_table("vfx_persistent.2da"));
        assert!(!is_ignored_table("classes.2da"));
    }

    #[test]
    fn test_is_priority_table() {
        assert!(is_priority_table("classes"));
        assert!(is_priority_table("feat"));
        assert!(!is_priority_table("cls_feat_fighter"));
    }

    #[test]
    fn test_should_load_table() {
        assert!(should_load_table("classes.2da"));
        assert!(should_load_table("feat.2da"));
        assert!(!should_load_table("ambientmusic.2da"));
    }
}
