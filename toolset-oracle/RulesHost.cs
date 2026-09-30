using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using NWN2Toolset.NWN2.Data.Blueprints;
using NWN2Toolset.NWN2.IO;
using NWN2Toolset.NWN2.Rules;
using OEIShared.IO;
using OEIShared.IO.GFF;
using OEIShared.IO.TwoDA;

namespace ToolsetOracle;

public static class RulesHost
{
    private const BindingFlags AnyStatic = BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic;

    /// Boots the toolset resource manager and the native rules library the way the toolset
    /// does. `NWN2RulesMgr` is internal; going through it keeps toolset code that calls
    /// `NWN2RulesMgr.GetRules()` on the same instance.
    ///
    /// `content` lists the save's mod content lowest priority first: `{"kind":"dir"|"erf","path":...}`.
    /// The toolset searches ERFs before directories and the newest repository first within a
    /// kind, so the caller sends module 2DAs as a directory and HAKs as the only ERFs.
    public static CNWRules Init(string install, Dictionary<string, object> request)
    {
        Directory.SetCurrentDirectory(install);
        new NWN2ResourceManager();
        ResourceManager.Instance.LoadStandardResources();
        if (request.TryGetValue("content", out var raw) && raw is IEnumerable layers)
        {
            foreach (var layer in layers)
            {
                if (layer is not Dictionary<string, object> map) throw new ArgumentException("content entries must be objects");
                var path = (string)map["path"];
                IResourceRepository repo = (string)map["kind"] switch
                {
                    "dir" => new DirectoryResourceRepository(path),
                    "erf" => new ERFResourceRepository(path),
                    var kind => throw new ArgumentException($"unknown content kind: {kind}"),
                };
                ResourceManager.Instance.AddRepository(repo);
            }
            TwoDAManager.Instance.Clear();
        }
        var mgr = typeof(NWN2ResourceManager).Assembly.GetType("NWN2Toolset.NWN2.Rules.NWN2RulesMgr", throwOnError: true)!;
        mgr.GetMethod("Initialize", AnyStatic)!.Invoke(null, null);
        return (CNWRules)mgr.GetMethod("GetRules", AnyStatic)!.Invoke(null, null)!;
    }

    public static readonly ATTRIBUTE[] Attributes =
    {
        ATTRIBUTE.STRENGTH, ATTRIBUTE.DEXTERITY, ATTRIBUTE.CONSTITUTION,
        ATTRIBUTE.INTELLIGENCE, ATTRIBUTE.WISDOM, ATTRIBUTE.CHARISMA,
    };

    public static readonly SAVING_THROW[] Saves =
    {
        SAVING_THROW.SAVING_THROW_FORTITUDE, SAVING_THROW.SAVING_THROW_REFLEX, SAVING_THROW.SAVING_THROW_WILL,
    };

    /// Loads the request's `.bic` into the engine's stats core. The toolset loader copies race,
    /// abilities, classes, feats, skill ranks and known spells; alignment is set from the file
    /// because prerequisites read it.
    public static NWN2_CreatureStatsCore LoadBic(Dictionary<string, object> request, out GFFStruct top)
    {
        var path = request.TryGetValue("bic", out var p) ? (string)p : throw new ArgumentException("bic is required");
        top = new GFFFile(path).TopLevelStruct;
        var blueprint = new NWN2CreatureBlueprint();
        blueprint.LoadEverythingFromGFFStruct(top, false);
        var core = blueprint.GetStatsCore();
        core.SetAlignmentGoodEvil(top.GetByteSafe("GoodEvil", 50));
        core.SetAlignmentLawChaos(top.GetByteSafe("LawfulChaotic", 50));
        core.SetIsPC(1);
        return core;
    }

    /// One engine level in `cls`: the first character level through `ResetToFirstLevelForPackage`,
    /// later ones through `LevelUpAutomatic` after the class prerequisites check.
    public static void LevelOnce(CNWRules rules, NWN2_CreatureStatsCore core, int cls, int package, bool first)
    {
        var classLevel = ClassLevel(core, cls);
        var characterLevel = core.GetLevel() + 1;
        if (classLevel == 0 && !first)
        {
            using var cnwClass = rules.GetClass((byte)cls);
            if (cnwClass == null || core.ClassPrereqsMet(cnwClass) == 0)
                throw new InvalidOperationException($"class {cls} prerequisites not met at level {characterLevel}");
        }
        var ok = first ? core.ResetToFirstLevelForPackage((byte)package) : core.LevelUpAutomatic((byte)cls, (byte)package);
        if (ok == 0 || ClassLevel(core, cls) != classLevel + 1)
            throw new InvalidOperationException($"engine did not level class {cls} with package {package} at level {characterLevel}");
    }

    public static int ClassLevel(NWN2_CreatureStatsCore core, int cls)
    {
        for (byte idx = 0; idx < core.GetNumClasses(); idx++)
            if (core.GetClass(idx) == cls) return core.GetClassLevel(idx);
        return 0;
    }

    /// Spell slots per spell level 0-9 of the class at `index`. `CalcMaxSpellsPerDayAtLevel` adds
    /// the prestige caster levels itself, so it takes the bare class level.
    public static List<int> SpellsPerDay(NWN2_CreatureStatsCore core, byte index)
    {
        var cls = core.GetClass(index);
        var classLevel = core.GetClassLevel(index);
        return Enumerable.Range(0, 10).Select(l => (int)core.CalcMaxSpellsPerDayAtLevel(cls, classLevel, (byte)l, 1)).ToList();
    }

    public static int[] BaseAbilities(NWN2_CreatureStatsCore core) =>
        Attributes.Select(a => (int)core.GetBaseAttribute(a)).ToArray();

    public static HashSet<int> Feats(NWN2_CreatureStatsCore core)
    {
        var feats = new HashSet<int>();
        for (ushort i = 0; i < core.GetNumberFeats(); i++) feats.Add(core.GetFeat(i));
        return feats;
    }

    /// The ability raised between two `BaseAbilities` snapshots, or -1.
    public static int AbilityIncrease(int[] before, int[] after) =>
        Enumerable.Range(0, after.Length).Where(i => after[i] > before[i]).DefaultIfEmpty(-1).First();

    public static int RequireInt(Dictionary<string, object> map, string key) =>
        OptInt(map, key) ?? throw new ArgumentException($"{key} is required");

    public static int? OptInt(Dictionary<string, object> map, string key) =>
        map.TryGetValue(key, out var v) && v != null ? Convert.ToInt32(v) : null;

    public static object Info(CNWRules rules) => new
    {
        classes = (int)rules.GetNumClasses(),
        races = (int)rules.GetNumRaces(),
        feats = (int)rules.GetNumFeats(),
        skills = (int)rules.GetNumSkills(),
        max_level = (int)rules.GetMaxLevel(),
    };
}
