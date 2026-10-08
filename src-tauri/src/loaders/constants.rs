const CHARACTER_PREFIXES: &[&str] = &[
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

/// Allow-list of the 2DAs the editor reads, by name prefix.
pub fn should_load_table(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    CHARACTER_PREFIXES
        .iter()
        .any(|prefix| name_lower.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_load_table() {
        assert!(should_load_table("classes.2da"));
        assert!(should_load_table("cls_feat_fighter.2da"));
        assert!(should_load_table("iprp_abilities.2da"));
        assert!(!should_load_table("ambientmusic.2da"));
    }

    #[test]
    fn test_pet_tables_load() {
        assert!(should_load_table("hen_companion.2da"));
        assert!(should_load_table("hen_familiar.2da"));
        assert!(!should_load_table("henchspells.2da"));
    }
}
