
namespace ToolsetBridge.Schema;

public sealed class ModuleVariableShape
{
    [JsonName("name")] public string Name { get; set; } = "";
    [JsonName("type")] public string Type { get; set; } = "";
    [JsonName("default")] public object? Default { get; set; }
    [JsonName("storage")] public string Storage { get; set; } = "";
}
