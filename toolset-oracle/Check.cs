using System;
using System.Collections.Generic;
using System.Linq;
using NWN2Toolset.NWN2.Rules;

namespace ToolsetOracle;

/// Loads a `.bic` into the engine's stats core and reports what the rules library computes.
/// Items are not modelled.
public static class Check
{
    public static object Run(CNWRules rules, Dictionary<string, object> request)
    {
        var levellingClass = RulesHost.RequireInt(request, "levelling_class");
        using var core = RulesHost.LoadBic(request, out _);

        var level = core.GetLevel();
        return new
        {
            level = (int)level,
            abilities = RulesHost.Attributes.Select(a => new
            {
                @base = (int)core.GetBaseAttribute(a),
                effective = (int)core.GetAttribute(a),
                modifier = core.GetAttributeModifier(a),
            }).ToList(),
            bab = core.CalcBaseAttackBonus(),
            saves = RulesHost.Saves.Select(s => new
            {
                @class = (int)core.CalcSavingThrowClassLevelMod(s),
                feat = core.CalcSavingThrowFeatMod(s),
                ability = (int)core.GetSavingThrowAbilityMod(s),
            }).ToList(),
            hp_max = core.GetMaxHitPoints(1),
            hp_hit_dice = core.CalcMaxHitpointsFromHitDice(),
            ac_stat = core.CalcACStatMods(),
            ac_dodge_feats = core.GetACDodgeModFromFeats(),
            skills = Enumerable.Range(0, rules.GetNumSkills()).Select(s => new
            {
                id = s,
                rank = (int)core.GetSkillRank((ushort)s),
                max_ranks = core.GetMaxSkillRanks((ushort)s, level, (byte)levellingClass),
                class_skill = core.IsSkillClassSkill((byte)levellingClass, (ushort)s) != 0,
            }).ToList(),
            casters = Enumerable.Range(0, core.GetNumClasses()).Select(i => Caster(core, (byte)i)).ToList(),
        };
    }

    private static object Caster(NWN2_CreatureStatsCore core, byte index)
    {
        var cls = core.GetClass(index);
        var classLevel = core.GetClassLevel(index);
        var bonus = core.CalcBonusCasterLevelsFromPrestigeClasses(cls);
        var effective = (byte)Math.Min(classLevel + bonus, 255);
        return new
        {
            @class = (int)cls,
            level = (int)classLevel,
            prestige_bonus = (int)bonus,
            spells_per_day = Enumerable.Range(0, 10).Select(l => (int)core.CalcMaxSpellsPerDayAtLevel(cls, effective, (byte)l, 0)).ToList(),
        };
    }
}
