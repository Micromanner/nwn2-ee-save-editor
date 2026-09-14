using System.Collections.Generic;

namespace ToolsetBridge.Schema;

public sealed class JournalShape
{
    [JsonName("categories")] public List<JournalCategory> Categories { get; set; } = new();
}

public sealed class JournalCategory
{
    [JsonName("tag")] public string Tag { get; set; } = "";
    [JsonName("name")] public string Name { get; set; } = "";
    [JsonName("priority")] public string Priority { get; set; } = "";
    [JsonName("xp")] public int Xp { get; set; }
    [JsonName("source")] public string Source { get; set; } = "";
    [JsonName("entries")] public List<JournalEntry> Entries { get; set; } = new();
}

public sealed class JournalEntry
{
    [JsonName("id")] public int Id { get; set; }
    [JsonName("text")] public string Text { get; set; } = "";
    [JsonName("final")] public bool Final { get; set; }
}
