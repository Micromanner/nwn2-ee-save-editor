using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using NWN2Toolset.NWN2.Rules;
using OEIShared.IO.TwoDA;

namespace ToolsetOracle;

/// Builds a character with the engine's own character creation (`ResetToFirstLevelForPackage`)
/// and level-up (`LevelUpAutomatic`), one level at a time. The per-level history is rebuilt by
/// diffing the core before and after each level instead of reading `CNWLevelStats` memory.
public static class Build
{
    private static readonly int[] DefaultAbilities = { 14, 14, 14, 14, 12, 10 };

    public sealed class LevelRecord
    {
        public int level { get; set; }
        public int @class { get; set; }
        public int hit_die { get; set; }
        public int ability_increase { get; set; }
        public int skill_points_remaining { get; set; }
        public int hp_after { get; set; }
        public List<int> feats_added { get; set; } = new();
        public List<int> skill_ranks_added { get; set; } = new();
        public List<List<int>> spells_added { get; set; } = new();
    }

    public sealed class ClassRecord
    {
        public int @class { get; set; }
        public int level { get; set; }
        public int domain1 { get; set; }
        public int domain2 { get; set; }
        public int school { get; set; }
        public List<int> spells_per_day { get; set; } = new();
        public List<List<int>> known { get; set; } = new();
    }

    public sealed class Result
    {
        public int race { get; set; }
        public int subrace { get; set; }
        public int package { get; set; }
        public int rules_classes { get; set; }
        public List<int> abilities { get; set; } = new();
        public List<int> effective_abilities { get; set; } = new();
        public int good_evil { get; set; }
        public int lawful_chaotic { get; set; }
        public int level { get; set; }
        public int bab { get; set; }
        public int hp_max { get; set; }
        public int hp_hit_dice { get; set; }
        public List<int> saves { get; set; } = new();
        public List<int> save_totals { get; set; } = new();
        public List<ClassRecord> classes { get; set; } = new();
        public List<int> feats { get; set; } = new();
        public List<int> skill_ranks { get; set; } = new();
        public List<LevelRecord> levels { get; set; } = new();
    }

    private sealed class Snapshot
    {
        public int[] Abilities = Array.Empty<int>();
        public HashSet<int> Feats = new();
        public int[] Skills = Array.Empty<int>();
        public Dictionary<int, List<int>[]> Known = new();
    }

    public static Result Run(CNWRules rules, Dictionary<string, object> request)
    {
        var race = RulesHost.RequireInt(request, "race");
        var subrace = RulesHost.RequireInt(request, "subrace");
        var abilities = OptInts(request, "abilities") ?? DefaultAbilities;
        if (abilities.Length != 6) throw new ArgumentException("abilities needs 6 values (Str Dex Con Int Wis Cha)");
        var goodEvil = RulesHost.OptInt(request, "good_evil") ?? 50;
        var lawfulChaotic = RulesHost.OptInt(request, "lawful_chaotic") ?? 50;
        var plan = ReadPlan(request);

        var classes2da = TwoDAManager.Instance.Get("classes");
        var packages2da = TwoDAManager.Instance.Get("packages");
        using var core = new NWN2_CreatureStatsCore();
        core.SetRace((ushort)race);
        core.SetSubRace((ushort)subrace);
        for (var i = 0; i < 6; i++) core.SetBaseAttribute(RulesHost.Attributes[i], (byte)abilities[i]);
        core.SetAlignmentGoodEvil((short)goodEvil);
        core.SetAlignmentLawChaos((short)lawfulChaotic);
        core.SetIsPC(1);

        var result = new Result { rules_classes = rules.GetNumClasses(), race = race, subrace = subrace, good_evil = goodEvil, lawful_chaotic = lawfulChaotic };
        var skillCount = (int)rules.GetNumSkills();
        var before = Take(core, skillCount);
        var first = true;
        int? previousPackage = null;
        foreach (var (cls, levels, packageOverride) in plan)
        {
            var package = packageOverride ?? OwnPackage(classes2da, packages2da, cls) ?? (first
                ? throw new ArgumentException($"class {cls} has no package of its own, so it cannot be the first class (the first level always takes the package's class)")
                : previousPackage!.Value);
            previousPackage = package;
            if (first) result.package = package;
            for (var i = 0; i < levels; i++)
            {
                RulesHost.LevelOnce(rules, core, cls, package, first);
                first = false;
                var after = Take(core, skillCount);
                result.levels.Add(Diff(core, before, after, cls, HitDie(classes2da, cls)));
                before = after;
            }
        }

        result.abilities = before.Abilities.ToList();
        result.effective_abilities = RulesHost.Attributes.Select(a => (int)core.GetAttribute(a)).ToList();
        result.level = core.GetLevel();
        result.bab = core.CalcBaseAttackBonus();
        result.hp_max = core.GetMaxHitPoints(1);
        result.hp_hit_dice = core.CalcMaxHitpointsFromHitDice();
        var saveAbilities = new[] { ATTRIBUTE.CONSTITUTION, ATTRIBUTE.DEXTERITY, ATTRIBUTE.WISDOM };
        result.saves = RulesHost.Saves.Select(s => (int)core.CalcSavingThrowClassLevelMod(s)).ToList();
        result.save_totals = result.saves.Select((save, i) => save + core.GetAttributeModifier(saveAbilities[i])).ToList();
        result.feats = before.Feats.OrderBy(f => f).ToList();
        result.skill_ranks = before.Skills.ToList();
        for (byte idx = 0; idx < core.GetNumClasses(); idx++)
        {
            var info = core.GetCoreClassInfo(idx);
            var cls = core.GetClass(idx);
            var classLevel = core.GetClassLevel(idx);
            result.classes.Add(new ClassRecord
            {
                @class = cls,
                level = classLevel,
                domain1 = info.GetDomain1(),
                domain2 = info.GetDomain2(),
                school = info.GetSchool(),
                spells_per_day = RulesHost.SpellsPerDay(core, idx),
                known = before.Known[idx].ToList(),
            });
        }
        return result;
    }

    private static Snapshot Take(NWN2_CreatureStatsCore core, int skillCount)
    {
        var snap = new Snapshot
        {
            Abilities = RulesHost.BaseAbilities(core),
            Feats = RulesHost.Feats(core),
            Skills = Enumerable.Range(0, skillCount).Select(s => (int)core.GetSkillRank((ushort)s)).ToArray(),
        };
        for (byte idx = 0; idx < core.GetNumClasses(); idx++)
        {
            var levels = new List<int>[10];
            for (byte lvl = 0; lvl < 10; lvl++)
            {
                levels[lvl] = new List<int>();
                var n = core.GetNumberKnownSpells(idx, lvl);
                for (byte k = 0; k < n; k++) levels[lvl].Add(core.GetKnownSpell(idx, lvl, k));
            }
            snap.Known[idx] = levels;
        }
        return snap;
    }

    private static LevelRecord Diff(NWN2_CreatureStatsCore core, Snapshot before, Snapshot after, int cls, int hitDie)
    {
        var record = new LevelRecord
        {
            level = core.GetLevel(),
            @class = cls,
            hit_die = hitDie,
            ability_increase = RulesHost.AbilityIncrease(before.Abilities, after.Abilities),
            skill_points_remaining = core.GetSkillPointsRemaining(),
            hp_after = core.GetMaxHitPoints(1),
            feats_added = after.Feats.Except(before.Feats).OrderBy(f => f).ToList(),
            skill_ranks_added = after.Skills.Select((rank, i) => rank - before.Skills[i]).ToList(),
            spells_added = Enumerable.Range(0, 10).Select(_ => new List<int>()).ToList(),
        };

        // A prestige level that advances casting records the base class's new spells in its own entry, like the game.
        for (byte idx = 0; idx < core.GetNumClasses(); idx++)
        {
            var had = before.Known.TryGetValue(idx, out var old) ? old : null;
            for (var lvl = 0; lvl < 10; lvl++)
                record.spells_added[lvl].AddRange(after.Known[idx][lvl].Except(had?[lvl] ?? new List<int>()));
        }
        return record;
    }

    /// The class's `classes.2da` package if that package belongs to the class, else the first
    /// package listing the class. Mod classes often point at an empty shared row; a package only
    /// steers feat, skill and spell picks, so the caller falls back to the previous class's.
    internal static int? OwnPackage(TwoDAFile classes, TwoDAFile packages, int cls)
    {
        var own = cls.ToString();
        if (int.TryParse(classes["Package"]?[cls], out var package) && package < packages.RowCount && packages["ClassID"][package] == own)
            return package;
        for (var i = 0; i < packages.RowCount; i++)
            if (packages["ClassID"][i] == own) return i;
        return null;
    }

    private static int HitDie(TwoDAFile classes, int cls) =>
        int.TryParse(classes["HitDie"]?[cls], out var die) ? die : 0;

    private static List<(int cls, int levels, int? package)> ReadPlan(Dictionary<string, object> request)
    {
        if (!request.TryGetValue("classes", out var raw) || raw is not IEnumerable entries)
            throw new ArgumentException("classes is required: [{\"class\":4,\"levels\":5}]");
        var plan = new List<(int, int, int?)>();
        foreach (var entry in entries)
        {
            if (entry is not Dictionary<string, object> map) throw new ArgumentException("classes entries must be objects");
            plan.Add((RulesHost.RequireInt(map, "class"), RulesHost.OptInt(map, "levels") ?? 1, RulesHost.OptInt(map, "package")));
        }
        if (plan.Count == 0) throw new ArgumentException("classes must not be empty");
        return plan;
    }

    private static int[]? OptInts(Dictionary<string, object> map, string key) =>
        map.TryGetValue(key, out var v) && v is IEnumerable items and not string
            ? items.Cast<object>().Select(Convert.ToInt32).ToArray()
            : null;
}
