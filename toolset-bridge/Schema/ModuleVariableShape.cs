using Newtonsoft.Json;

namespace ToolsetBridge.Schema;

public sealed class ModuleVariableShape
{
    [JsonProperty("name")] public string Name { get; set; } = "";
    [JsonProperty("type")] public string Type { get; set; } = "";
    [JsonProperty("default")] public object? Default { get; set; }
    [JsonProperty("storage")] public string Storage { get; set; } = "";
}
