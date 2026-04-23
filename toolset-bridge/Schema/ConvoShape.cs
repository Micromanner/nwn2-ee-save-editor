using System.Collections.Generic;
using Newtonsoft.Json;

namespace ToolsetBridge.Schema;

public sealed class ConvoShape
{
    [JsonProperty("nodes")] public List<ConvoNode> Nodes { get; set; } = new();
}

public sealed class ConvoNode
{
    [JsonProperty("dlg")] public string Dlg { get; set; } = "";
    [JsonProperty("node")] public int Node { get; set; }
    [JsonProperty("speaker")] public string Speaker { get; set; } = "";
    [JsonProperty("text_strref")] public int TextStrref { get; set; }
    [JsonProperty("actions")] public List<ConvoFunctorEntry> Actions { get; set; } = new();
    [JsonProperty("conditions")] public List<ConvoFunctorEntry> Conditions { get; set; } = new();
}

public sealed class ConvoFunctorEntry
{
    [JsonProperty("kind")] public string Kind { get; set; } = "";
    [JsonProperty("script")] public string Script { get; set; } = "";
    [JsonProperty("params")] public object[] Params { get; set; } = System.Array.Empty<object>();
}
