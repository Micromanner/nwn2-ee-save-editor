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
    // NWN2ConversationConnectorType: StartingEntry, Entry (NPC), Reply (player).
    [JsonProperty("type")] public string Type { get; set; } = "";
    // False for empty "continue" replies and NPC lines without text.
    [JsonProperty("has_text")] public bool HasText { get; set; }
    // Connector ids of the lines that can follow this one, in authored order.
    [JsonProperty("children")] public List<int> Children { get; set; } = new();
    [JsonProperty("actions")] public List<ConvoFunctorEntry> Actions { get; set; } = new();
    [JsonProperty("conditions")] public List<ConvoFunctorEntry> Conditions { get; set; } = new();
}

public sealed class ConvoFunctorEntry
{
    [JsonProperty("kind")] public string Kind { get; set; } = "";
    [JsonProperty("script")] public string Script { get; set; } = "";
    [JsonProperty("params")] public object[] Params { get; set; } = System.Array.Empty<object>();
    // Toolset "Not" checkbox on a condition: the script result is inverted.
    [JsonProperty("negate")] public bool Negate { get; set; }
    // NWN2ConditionalType ("and"/"or"): how this condition combines with the previous one.
    [JsonProperty("join")] public string Join { get; set; } = "";
}
