using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using HarmonyLib;
using NWN2Toolset.NWN2.Data;
using OEIShared.IO;
using OEIShared.Utils;

namespace ToolsetBridge;

// The toolset's module open path is wired through NWN2ToolsetMainForm.App (for the property
// grid refresh and ContentManager.InitializeModule), which is null in a headless process.
// We Harmony-patch the two entry points that touch it:
//   1. NWN2ModuleInformation.set_CustomTalkFile   — assigns backing field only.
//   2. NWN2GameModule.OEIUnserialize(string)      — reimplemented minus blueprints and ContentManager.
//
// The bridge never needs blueprints or the content manager; everything we emit is read from
// Journal, FactionData, Conversations, and ModuleInformation directly.
internal static class GuiStubPatches
{
    private const string HarmonyId = "nwn2_ee_editor.toolset_bridge.gui_stubs";
    private static bool s_applied;

    public static void Apply()
    {
        if (s_applied) return;
        var h = new Harmony(HarmonyId);

        PatchSetterToFieldAssign(h, typeof(NWN2ModuleInformation), "CustomTalkFile", "m_cCustomTalkFile");
        PatchModuleOpen(h);

        s_applied = true;
    }

    // --- Setter -> backing-field only ---------------------------------------------------------

    private static readonly Dictionary<MethodBase, string> s_backingFieldBySetter = new();

    private static void PatchSetterToFieldAssign(Harmony h, Type declaring, string propertyName, string backingField)
    {
        var setter = declaring.GetProperty(propertyName, BindingFlags.Public | BindingFlags.Instance)?.GetSetMethod()
            ?? throw new InvalidOperationException($"setter not found: {declaring.FullName}.{propertyName}");
        if (declaring.GetField(backingField, BindingFlags.NonPublic | BindingFlags.Instance) == null)
            throw new InvalidOperationException($"backing field not found: {declaring.FullName}.{backingField}");

        s_backingFieldBySetter[setter] = backingField;
        var prefix = new HarmonyMethod(
            typeof(GuiStubPatches).GetMethod(nameof(SetterToFieldPrefix), BindingFlags.NonPublic | BindingFlags.Static)!);
        h.Patch(setter, prefix);
    }

    private static bool SetterToFieldPrefix(object __instance, object value, MethodBase __originalMethod)
    {
        if (!s_backingFieldBySetter.TryGetValue(__originalMethod, out var fieldName)) return true;
        var field = __instance.GetType().GetField(fieldName, BindingFlags.NonPublic | BindingFlags.Instance);
        field?.SetValue(__instance, value);
        return false;
    }

    // --- NWN2GameModule.OEIUnserialize(string) replacement -----------------------------------

    private static void PatchModuleOpen(Harmony h)
    {
        var target = typeof(NWN2GameModule).GetMethod(
            "OEIUnserialize",
            BindingFlags.Public | BindingFlags.Instance,
            binder: null,
            types: new[] { typeof(string) },
            modifiers: null)
            ?? throw new InvalidOperationException("NWN2GameModule.OEIUnserialize(string) not found");

        var prefix = new HarmonyMethod(
            typeof(GuiStubPatches).GetMethod(nameof(ModuleOpenPrefix), BindingFlags.NonPublic | BindingFlags.Static)!);
        h.Patch(target, prefix);
    }

    // Replaces NWN2GameModule.OEIUnserialize(string) with a GUI-free equivalent.
    // Must match the original side-effects needed by downstream adapters: m_sName, m_sFileName,
    // m_cFile.Open, m_cModuleInfo loaded, m_cJournal loaded, m_cFactionData loaded. We skip:
    //   - LoadBlueprints (not needed by bridge)
    //   - ContentManager.InitializeModule (requires NWN2ToolsetMainForm.App)
    //   - the ModuleExpanded event (no subscribers in bridge process)
    private static bool ModuleOpenPrefix(NWN2GameModule __instance, string sFilename)
    {
        var t = typeof(NWN2GameModule);
        var fName = t.GetField("m_sName", BindingFlags.NonPublic | BindingFlags.Instance)!;
        var fFileName = t.GetField("m_sFileName", BindingFlags.NonPublic | BindingFlags.Instance)!;
        var fFile = t.GetField("m_cFile", BindingFlags.NonPublic | BindingFlags.Instance)!;
        var fGRCData = t.GetField("m_cGRCData", BindingFlags.NonPublic | BindingFlags.Instance)!;
        var fModInfo = t.GetField("m_cModuleInfo", BindingFlags.NonPublic | BindingFlags.Instance)!;
        var fJournal = t.GetField("m_cJournal", BindingFlags.NonPublic | BindingFlags.Instance)!;
        var fFactionData = t.GetField("m_cFactionData", BindingFlags.NonPublic | BindingFlags.Instance)!;

        fName.SetValue(__instance, Path.GetFileNameWithoutExtension(sFilename));
        fFileName.SetValue(__instance, sFilename);

        var file = fFile.GetValue(__instance)!;
        file.GetType().GetMethod("Open", new[] { typeof(string) })!.Invoke(file, new object[] { sFilename });

        // Setting GRCData.Repository triggers PopulateContents, which scans the module's
        // resource repository and fills the Areas / Conversations / Scripts dictionaries.
        // This is the side-effect we need so NWN2GameModule.Conversations is non-empty.
        var grcData = fGRCData.GetValue(__instance)!;
        var fileRepo = file.GetType().GetProperty("Repository")!.GetValue(file);
        var asDirRepo = fileRepo as OEIShared.IO.DirectoryResourceRepository;
        grcData.GetType().GetProperty("Repository")!.SetValue(grcData, asDirRepo);

        var dirName = (string)file.GetType().GetProperty("DirectoryName")!.GetValue(file)!;

        var modInfo = fModInfo.GetValue(__instance)!;
        modInfo.GetType().GetMethod("OEIUnserialize", new[] { typeof(string) })!
            .Invoke(modInfo, new object[] { Path.Combine(dirName, "module.ifo") });

        // Journal: load from module.jrl if present in the expanded dir; otherwise skip (bridge
        // can emit empty journal). The original would serialize a fresh .jrl and re-register the
        // repo entry, but we're read-only.
        var journalPath = Path.Combine(dirName, "module.jrl");
        if (File.Exists(journalPath))
        {
            var journal = fJournal.GetValue(__instance)!;
            journal.GetType().GetMethod("OEIUnserialize", new[] { typeof(string) })!
                .Invoke(journal, new object[] { journalPath });
        }

        var repute = Path.Combine(dirName, "repute.fac");
        if (File.Exists(repute))
        {
            var factionData = fFactionData.GetValue(__instance)!;
            factionData.GetType().GetMethod("OEIUnserialize", new[] { typeof(string) })!
                .Invoke(factionData, new object[] { repute });
        }

        return false;
    }
}
