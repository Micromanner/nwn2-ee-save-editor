using System.Collections.Generic;
using Newtonsoft.Json;

namespace ToolsetBridge.Schema;

public sealed class JournalShape
{
    [JsonProperty("categories")] public List<JournalCategory> Categories { get; set; } = new();
}

public sealed class JournalCategory
{
    [JsonProperty("tag")] public string Tag { get; set; } = "";
    [JsonProperty("name")] public string Name { get; set; } = "";
    [JsonProperty("priority")] public string Priority { get; set; } = "";
    [JsonProperty("xp")] public int Xp { get; set; }
    [JsonProperty("source")] public string Source { get; set; } = "";
    [JsonProperty("entries")] public List<JournalEntry> Entries { get; set; } = new();
}

public sealed class JournalEntry
{
    [JsonProperty("id")] public int Id { get; set; }
    [JsonProperty("text")] public string Text { get; set; } = "";
    [JsonProperty("final")] public bool Final { get; set; }
}
