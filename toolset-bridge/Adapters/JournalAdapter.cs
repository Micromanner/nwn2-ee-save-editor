using System;
using System.Collections.Generic;
using System.IO;
using OEIShared.IO.GFF;
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

    /// Parses a standalone `.jrl` (campaign folder `module.jrl`) as plain GFF. Campaigns
    /// keep their quest journal here, not inside the member modules, so a module-only
    /// read yields no categories for the OC/MotB/SoZ. TLK-backed text resolves only when
    /// the talk table is open (serve mode); embedded strings resolve regardless.
    public static JournalShape ReadFile(string jrlPath, string source)
    {
        var shape = new JournalShape();
        if (!File.Exists(jrlPath)) return shape;

        var gff = new GFFFile(jrlPath);
        var categories = gff.TopLevelStruct.GetListSafe("Categories");
        if (categories == null) return shape;

        foreach (GFFStruct cat in categories.StructList)
        {
            var tag = cat.GetStringSafe("Tag") ?? "";
            var priority = (NWN2Toolset.NWN2.Data.Journal.NWN2JournalPriority)Number(cat, "Priority");
            var category = new JournalCategory
            {
                Tag = tag,
                Name = LocString(cat, "Name", tag),
                Priority = priority.ToString(),
                Xp = Number(cat, "XP"),
                Source = source,
                Entries = new List<JournalEntry>(),
            };

            var entries = cat.GetListSafe("EntryList");
            if (entries != null)
            {
                foreach (GFFStruct entry in entries.StructList)
                {
                    category.Entries.Add(new JournalEntry
                    {
                        Id = Number(entry, "ID"),
                        Text = LocString(entry, "Text", ""),
                        Final = Number(entry, "End") != 0,
                    });
                }
            }

            shape.Categories.Add(category);
        }

        return shape;
    }

    private static int Number(GFFStruct s, string field)
    {
        var f = s.GetFieldSafe(field);
        if (f == null) return 0;
        try { return Convert.ToInt32(f.Value); }
        catch { return 0; }
    }

    private static string LocString(GFFStruct s, string field, string fallback)
    {
        try
        {
            var text = s.GetExoLocStringSafe(field)?.ToString();
            return string.IsNullOrEmpty(text) ? fallback : text;
        }
        catch
        {
            return fallback;
        }
    }
}
