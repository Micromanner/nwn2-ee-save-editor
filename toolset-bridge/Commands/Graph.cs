using Newtonsoft.Json;
using ToolsetBridge.Adapters;
using ToolsetBridge.Schema;

namespace ToolsetBridge.Commands;

public static class Graph
{
    public static int Run(LoadedContext ctx, string outPath)
    {
        var output = Compute(ctx);
        var json = Timing.Measure("serialize", () => JsonConvert.SerializeObject(output, Formatting.Indented));
        Timing.Measure("write_out", () => CommandOutput.Write(outPath, json));
        return ExitCodes.Ok;
    }

    /// Build the reference graph for an already-loaded module. Each adapter is timed
    /// so per-request breakdowns show up on stderr alongside a one-line `[info]`.
    public static BridgeOutput Compute(LoadedContext ctx)
    {
        var output = new BridgeOutput
        {
            Module = new ModuleInfo { Path = ctx.RootPath, Name = ctx.Name, Haks = ctx.Haks },
            Journal = Timing.Measure("journal", () => JournalAdapter.Read(ctx)),
            Factions = Timing.Measure("factions", () => FactionAdapter.Read(ctx)),
            ModuleVariables = Timing.Measure("module_vars", () => ModuleVariableAdapter.Read(ctx)),
            Convo = Timing.Measure("convo", () => ConversationAdapter.Read(ctx)),
        };
        Log.Info($"convo_nodes={output.Convo.Nodes.Count} journal_categories={output.Journal.Categories.Count}");
        return output;
    }
}
