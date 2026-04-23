using System.Collections.Generic;
using Newtonsoft.Json;

namespace ToolsetBridge.Schema;

public sealed class FactionShape
{
    [JsonProperty("id")] public int Id { get; set; }
    [JsonProperty("name")] public string Name { get; set; } = "";
    [JsonProperty("parent_id")] public int ParentId { get; set; }
    [JsonProperty("reputations")] public List<FactionRep> Reputations { get; set; } = new();
    [JsonProperty("source")] public string Source { get; set; } = "";
}

public sealed class FactionRep
{
    [JsonProperty("faction_id")] public int FactionId { get; set; }
    [JsonProperty("rep")] public int Rep { get; set; }
}
