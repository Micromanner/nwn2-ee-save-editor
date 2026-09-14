using System.Collections.Generic;

namespace ToolsetBridge.Schema;

public sealed class ConvoShape
{
    [JsonName("nodes")] public List<ConvoNode> Nodes { get; set; } = new();
}

public sealed class ConvoNode
{
    [JsonName("dlg")] public string Dlg { get; set; } = "";
    [JsonName("node")] public int Node { get; set; }
    [JsonName("speaker")] public string Speaker { get; set; } = "";
    [JsonName("text_strref")] public int TextStrref { get; set; }
    // NWN2ConversationConnectorType: StartingEntry, Entry (NPC), Reply (player).
    [JsonName("type")] public string Type { get; set; } = "";
    // False for empty "continue" replies and NPC lines without text.
    [JsonName("has_text")] public bool HasText { get; set; }
    // Connector ids of the lines that can follow this one, in authored order.
    [JsonName("children")] public List<int> Children { get; set; } = new();
    [JsonName("actions")] public List<ConvoFunctorEntry> Actions { get; set; } = new();
    [JsonName("conditions")] public List<ConvoFunctorEntry> Conditions { get; set; } = new();
}

public sealed class ConvoFunctorEntry
{
    [JsonName("kind")] public string Kind { get; set; } = "";
    [JsonName("script")] public string Script { get; set; } = "";
    [JsonName("params")] public object[] Params { get; set; } = System.Array.Empty<object>();
    // Toolset "Not" checkbox on a condition: the script result is inverted.
    [JsonName("negate")] public bool Negate { get; set; }
    // NWN2ConditionalType ("and"/"or"): how this condition combines with the previous one.
    [JsonName("join")] public string Join { get; set; } = "";
}
