using System.Collections.Generic;

namespace ToolsetBridge.Schema;

public sealed class FactionShape
{
    [JsonName("id")] public int Id { get; set; }
    [JsonName("name")] public string Name { get; set; } = "";
    [JsonName("parent_id")] public int ParentId { get; set; }
    [JsonName("reputations")] public List<FactionRep> Reputations { get; set; } = new();
    [JsonName("source")] public string Source { get; set; } = "";
}

public sealed class FactionRep
{
    [JsonName("faction_id")] public int FactionId { get; set; }
    [JsonName("rep")] public int Rep { get; set; }
}
