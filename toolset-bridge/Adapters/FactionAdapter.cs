using System.Collections.Generic;
using ToolsetBridge.Schema;

namespace ToolsetBridge.Adapters;

public static class FactionAdapter
{
    public static List<FactionShape> Read(LoadedContext ctx)
    {
        var result = new List<FactionShape>();
        if (ctx.Module?.FactionData?.Factions == null) return result;

        var data = ctx.Module.FactionData;
        var factionList = new List<NWN2Toolset.NWN2.Data.Factions.NWN2Faction>();
        foreach (var obj in data.Factions)
        {
            if (obj is NWN2Toolset.NWN2.Data.Factions.NWN2Faction f)
                factionList.Add(f);
        }

        foreach (var src in factionList)
        {
            var shape = new FactionShape
            {
                Id = (int)src.ID,
                Name = src.Name ?? "",
                ParentId = 0,
                Source = ctx.Source,
                Reputations = new List<FactionRep>(),
            };

            foreach (var tgt in factionList)
            {
                if (tgt.ID == src.ID) continue;
                var rep = data.GetStanding((int)src.ID, (int)tgt.ID);
                shape.Reputations.Add(new FactionRep { FactionId = (int)tgt.ID, Rep = rep });
            }

            result.Add(shape);
        }

        return result;
    }
}
