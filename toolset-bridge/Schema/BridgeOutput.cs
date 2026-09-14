using System.Collections.Generic;

namespace ToolsetBridge.Schema;

public sealed class BridgeOutput
{
    [JsonName("module")] public ModuleInfo Module { get; set; } = new();
    [JsonName("journal")] public JournalShape Journal { get; set; } = new();
    [JsonName("factions")] public List<FactionShape> Factions { get; set; } = new();
    [JsonName("module_variables")] public List<ModuleVariableShape> ModuleVariables { get; set; } = new();
    [JsonName("convo")] public ConvoShape Convo { get; set; } = new();
}

public sealed class ModuleInfo
{
    [JsonName("path")] public string Path { get; set; } = "";
    [JsonName("name")] public string Name { get; set; } = "";
    [JsonName("haks")] public IReadOnlyList<string> Haks { get; set; } = System.Array.Empty<string>();
}
