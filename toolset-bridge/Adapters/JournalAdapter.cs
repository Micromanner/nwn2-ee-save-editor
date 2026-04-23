using System;
using System.Collections.Generic;
using ToolsetBridge.Schema;

namespace ToolsetBridge.Adapters;

public static class JournalAdapter
{
    public static JournalShape Read(LoadedContext ctx)
    {
        var shape = new JournalShape();
        if (ctx.Module?.Journal?.Categories == null) return shape;

        foreach (var obj in ctx.Module.Journal.Categories)
        {
            if (obj is not NWN2Toolset.NWN2.Data.Journal.NWN2JournalCategory cat)
                continue;

            var category = new JournalCategory
            {
                Tag = cat.Tag ?? "",
                Name = cat.Name?.ToString() ?? cat.Tag ?? "",
                Priority = cat.Priority.ToString(),
                Xp = (int)cat.XP,
                Source = ctx.Source,
                Entries = new List<JournalEntry>(),
            };

            foreach (var entryObj in cat.Entries)
            {
                if (entryObj is not NWN2Toolset.NWN2.Data.Journal.NWN2JournalEntry entry)
                    continue;

                category.Entries.Add(new JournalEntry
                {
                    Id = (int)entry.ID,
                    Text = entry.Text?.ToString() ?? "",
                    Final = entry.Endpoint,
                });
            }

            shape.Categories.Add(category);
        }

        return shape;
    }
}
