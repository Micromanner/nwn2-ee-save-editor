using System.Collections.Generic;
using Newtonsoft.Json;

namespace ToolsetBridge.Schema;

public sealed class BridgeOutput
{
    [JsonProperty("module")] public ModuleInfo Module { get; set; } = new();
    [JsonProperty("journal")] public JournalShape Journal { get; set; } = new();
    [JsonProperty("factions")] public List<FactionShape> Factions { get; set; } = new();
    [JsonProperty("module_variables")] public List<ModuleVariableShape> ModuleVariables { get; set; } = new();
    [JsonProperty("convo")] public ConvoShape Convo { get; set; } = new();
}

public sealed class ModuleInfo
{
    [JsonProperty("path")] public string Path { get; set; } = "";
    [JsonProperty("name")] public string Name { get; set; } = "";
    [JsonProperty("haks")] public IReadOnlyList<string> Haks { get; set; } = System.Array.Empty<string>();
}
