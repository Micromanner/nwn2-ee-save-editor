using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Newtonsoft.Json;
using OEIShared.IO.GFF;

namespace ToolsetBridge.Commands;

public static class ListModules
{
    // Resolves a campaign.cam (either given directly or via its folder) and emits the
    // ordered list of module names it references, plus its start module. Pure GFF parse —
    // no ResourceManager, TLK, or Harmony init required.
    public static int Run(string campaignPath, string nwn2Install, string outPath)
    {
        string camFile;
        string camFolder;
        if (File.Exists(campaignPath) &&
            string.Equals(Path.GetExtension(campaignPath), ".cam", StringComparison.OrdinalIgnoreCase))
        {
            camFile = campaignPath;
            camFolder = Path.GetDirectoryName(campaignPath) ?? "";
        }
        else if (Directory.Exists(campaignPath))
        {
            camFolder = campaignPath;
            camFile = Path.Combine(campaignPath, "campaign.cam");
            if (!File.Exists(camFile))
            {
                Log.Error($"no campaign.cam under {campaignPath}");
                return ExitCodes.LoadFailed;
            }
        }
        else
        {
            Log.Error($"campaign path not found: {campaignPath}");
            return ExitCodes.LoadFailed;
        }

        var gff = new GFFFile(camFile);
        var root = gff.TopLevelStruct;

        var result = new CampaignModulesShape
        {
            CampaignPath = camFolder,
            CampaignFile = camFile,
            DisplayName = root.GetStringSafe("DisplayName"),
            StartModule = root.GetStringSafe("StartModule"),
            JournalSynch = GetInt(root, "JournalSynch") != 0,
            Modules = new List<CampaignModuleEntry>(),
        };

        var modNames = root.GetListSafe("ModNames");
        if (modNames != null)
        {
            foreach (GFFStruct mod in modNames.StructList)
            {
                var name = mod.GetStringSafe("ModuleName");
                if (string.IsNullOrEmpty(name)) continue;

                // Resolve .mod path. NWN2 searches in this order: campaign folder (overrides),
                // then the install's modules/. Match that so the editor can spawn `graph` on
                // the right .mod and respect campaign overrides.
                var (resolved, kind) = ResolveModulePath(name, camFolder, nwn2Install);
                result.Modules.Add(new CampaignModuleEntry
                {
                    Name = name,
                    ResolvedPath = resolved,
                    ResolutionKind = kind,
                });
            }
        }

        var json = JsonConvert.SerializeObject(result, Formatting.Indented);
        CommandOutput.Write(outPath, json);
        return ExitCodes.Ok;
    }

    private static int GetInt(GFFStruct s, string name)
    {
        var f = s.GetFieldSafe(name);
        if (f == null) return 0;
        try { return Convert.ToInt32(f.Value); }
        catch { return 0; }
    }

    private static (string path, string kind) ResolveModulePath(string name, string camFolder, string nwn2Install)
    {
        // Campaign-local .mod (override / companion modules). Case-insensitive filesystem
        // on Windows, so Path.Combine + File.Exists is sufficient.
        var camLocal = Path.Combine(camFolder, name + ".mod");
        if (File.Exists(camLocal)) return (camLocal, "campaign");

        if (!string.IsNullOrWhiteSpace(nwn2Install))
        {
            var installed = Path.Combine(nwn2Install, "modules", name + ".mod");
            if (File.Exists(installed)) return (installed, "install");
        }

        return ("", "unresolved");
    }
}

public sealed class CampaignModulesShape
{
    [JsonProperty("campaign_path")] public string CampaignPath { get; set; } = "";
    [JsonProperty("campaign_file")] public string CampaignFile { get; set; } = "";
    [JsonProperty("display_name")] public string DisplayName { get; set; } = "";
    [JsonProperty("start_module")] public string StartModule { get; set; } = "";
    [JsonProperty("journal_synch")] public bool JournalSynch { get; set; }
    [JsonProperty("modules")] public List<CampaignModuleEntry> Modules { get; set; } = new();
}

public sealed class CampaignModuleEntry
{
    [JsonProperty("name")] public string Name { get; set; } = "";
    [JsonProperty("resolved_path")] public string ResolvedPath { get; set; } = "";
    // "campaign" = found in campaign folder, "unresolved" = not found locally.
    [JsonProperty("resolution_kind")] public string ResolutionKind { get; set; } = "unresolved";
}
