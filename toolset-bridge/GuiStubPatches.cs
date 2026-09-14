using System;
using System.Reflection;
using NWN2Toolset.NWN2.Data;

namespace ToolsetBridge;

// NWN2ModuleInformation.set_CustomTalkFile ends in
// NWN2ToolsetMainForm.App.DefaultPropertyGrid.UpdateStringRefs(), two nulls deep in a
// headless process, and NWN2ModuleInformation.OEIUnserialize assigns the property
// unconditionally while reading Mod_CustomTlk. The detour replaces the setter with a
// plain backing-field assignment; the bridge never reads CustomTalkFile.
// The module open path itself is not patched: see HeadlessModuleOpen.
internal static class GuiStubPatches
{
    private static bool s_applied;

    private static readonly FieldInfo? s_customTalkFile =
        typeof(NWN2ModuleInformation).GetField("m_cCustomTalkFile", BindingFlags.NonPublic | BindingFlags.Instance);

    public static void Apply()
    {
        if (s_applied) return;

        var setter = typeof(NWN2ModuleInformation)
            .GetProperty("CustomTalkFile", BindingFlags.Public | BindingFlags.Instance)?
            .GetSetMethod()
            ?? throw new InvalidOperationException("setter not found: NWN2ModuleInformation.CustomTalkFile");
        if (s_customTalkFile == null)
            throw new InvalidOperationException("backing field not found: NWN2ModuleInformation.m_cCustomTalkFile");

        var replacement = typeof(GuiStubPatches)
            .GetMethod(nameof(CustomTalkFileSetter), BindingFlags.NonPublic | BindingFlags.Static)!;
        Detour.Install(setter, replacement);

        s_applied = true;
    }

    private static void CustomTalkFileSetter(NWN2ModuleInformation self, string value)
    {
        s_customTalkFile!.SetValue(self, value);
    }
}
