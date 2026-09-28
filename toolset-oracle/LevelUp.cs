using System;
using System.Collections.Generic;
using System.Linq;
using NWN2Toolset.NWN2.Rules;
using OEIShared.IO.TwoDA;

namespace ToolsetOracle;

/// Levels a `.bic` up with the engine's `LevelUpAutomatic`, one level at a time. The package
/// makes the free choices (feats, skills, ability increase), so callers compare only what those
/// choices cannot change, plus whether the editor's automatic feats are among `feats_added`.
public static class LevelUp
{
    public sealed class Step
    {
        public int level { get; set; }
        public int skill_points_gained { get; set; }
        public int hp_after { get; set; }
        public int ability_increase { get; set; }
        public int bab { get; set; }
        public List<int> saves { get; set; } = new();
        public List<int> feats_added { get; set; } = new();
    }

    public static object Run(CNWRules rules, Dictionary<string, object> request)
    {
        var cls = RulesHost.RequireInt(request, "class");
        var levels = RulesHost.OptInt(request, "levels") ?? 1;
        using var core = RulesHost.LoadBic(request, out var top);

        var package = RulesHost.OptInt(request, "package")
            ?? Build.OwnPackage(TwoDAManager.Instance.Get("classes"), TwoDAManager.Instance.Get("packages"), cls)
            ?? top.GetByteSafe("StartingPackage", 0);

        var steps = new List<Step>();
        for (var i = 0; i < levels; i++)
        {
            var characterLevel = core.GetLevel() + 1;
            var gain = core.ComputeSkillPointGain((byte)cls, (byte)characterLevel, 0);
            var featsBefore = RulesHost.Feats(core);
            var abilitiesBefore = RulesHost.BaseAbilities(core);
            RulesHost.LevelOnce(rules, core, cls, package, first: false);
            steps.Add(new Step
            {
                level = characterLevel,
                skill_points_gained = gain,
                hp_after = core.GetMaxHitPoints(1),
                ability_increase = RulesHost.AbilityIncrease(abilitiesBefore, RulesHost.BaseAbilities(core)),
                bab = core.CalcBaseAttackBonus(),
                saves = RulesHost.Saves.Select(s => (int)core.CalcSavingThrowClassLevelMod(s)).ToList(),
                feats_added = RulesHost.Feats(core).Except(featsBefore).OrderBy(f => f).ToList(),
            });
        }
        return new { package, steps };
    }
}
