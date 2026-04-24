using System;
using System.IO;
using System.Linq;
using NWN2Toolset.NWN2.Data;
using NWN2Toolset.NWN2.IO;
using OEIShared.IO;
using OEIShared.IO.TalkTable;
using OEIShared.Utils;

namespace ToolsetBridge.Adapters;

public sealed class LoadedContext
{
    public string RootPath { get; init; } = "";
    public string Name { get; init; } = "";
    public string[] Haks { get; init; } = Array.Empty<string>();
    public NWN2GameModule? Module { get; init; }

    // Campaign override resolution deferred (see README). Always "module" in v1.
    public string Source { get; init; } = "module";
}

public static class ToolsetLoader
{
    private static bool s_resourceManagerReady;

    public static void EnsureResourceManager(string nwn2Install)
    {
        if (s_resourceManagerReady) return;

        if (!Directory.Exists(nwn2Install))
            throw new DirectoryNotFoundException($"NWN2 install not found: {nwn2Install}");

        // NWN2ResourceManager reads BaseDirectory from CWD at ctor time, and
        // LoadStandardResources() walks BaseDirectory/Data/*.zip relative to it.
        Directory.SetCurrentDirectory(nwn2Install);

        Timing.Measure("mgr_ctor", () =>
        {
            if (ResourceManager.Instance == null)
                _ = new NWN2ResourceManager(); // self-assigns ResourceManager.ms_cInstance
        });

        Timing.Measure("load_std_resources", () => NWN2ResourceManager.Instance.LoadStandardResources());

        // Load the base TLK so OEIExoLocString.ToString() resolves TLK string refs.
        // Without this, all journal/convo text comes back empty. English-only; per-language
        // support and custom TLKs (module.ifo Mod_CustomTlk) would layer on top.
        Timing.Measure("tlk_open", () =>
        {
            TalkTable.Instance.TalkTableDirectory = nwn2Install;
            BWLanguages.CurrentLanguage = BWLanguages.BWLanguage.English;
            TalkTable.Instance.Open(BWLanguages.BWLanguage.English);
        });

        // ModuleInfo setters reach NWN2ToolsetMainForm.App.DefaultPropertyGrid from
        // OEIUnserialize. Those code paths are patched out in GuiStubPatches so they
        // just assign the backing private field and return. No MainForm needed.
        Timing.Measure("harmony_patch", () => GuiStubPatches.Apply());

        s_resourceManagerReady = true;
    }

    public static LoadedContext LoadModule(string modulePath)
    {
        var isDir = Directory.Exists(modulePath);
        var isFile = !isDir && File.Exists(modulePath);
        if (!isDir && !isFile)
            throw new FileNotFoundException($"module path not found: {modulePath}");

        var module = new NWN2GameModule(bDoNothing: true);
        if (isFile)
            module.OpenModuleFile(modulePath);
        else
            module.OpenModuleDirectory(modulePath);

        var haks = module.ModuleInfo?.HakPaks?
            .Cast<object>()
            .Select(h => h.ToString() ?? "")
            .Where(s => !string.IsNullOrEmpty(s))
            .ToArray() ?? Array.Empty<string>();

        return new LoadedContext
        {
            RootPath = modulePath,
            Name = module.Name,
            Haks = haks,
            Module = module,
            Source = "module",
        };
    }

    public static LoadedContext LoadCampaign(string campaignPath)
    {
        // v2: campaign loading requires working around NWN2CampaignManager.ActiveCampaign GUI deps.
        // For v1 we tell the caller to point at a specific module within the campaign.
        throw new NotImplementedException(
            "campaign loading deferred to v2 — pass --module <path to .mod or expanded module dir> instead. See README 'Campaign (deferred)'.");
    }
}
