using Newtonsoft.Json;
using ToolsetBridge.Adapters;
using ToolsetBridge.Schema;

namespace ToolsetBridge.Commands;

public static class Graph
{
    public static int Run(LoadedContext ctx, string outPath)
    {
        var output = new BridgeOutput
        {
            Module = new ModuleInfo { Path = ctx.RootPath, Name = ctx.Name, Haks = ctx.Haks },
            Journal = JournalAdapter.Read(ctx),
            Factions = FactionAdapter.Read(ctx),
            ModuleVariables = ModuleVariableAdapter.Read(ctx),
            Convo = ConversationAdapter.Read(ctx),
        };

        var json = JsonConvert.SerializeObject(output, Formatting.Indented);
        CommandOutput.Write(outPath, json);
        return ExitCodes.Ok;
    }
}
