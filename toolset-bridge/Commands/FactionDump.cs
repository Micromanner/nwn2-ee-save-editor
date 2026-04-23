using Newtonsoft.Json;
using ToolsetBridge.Adapters;

namespace ToolsetBridge.Commands;

public static class FactionDump
{
    public static int Run(LoadedContext ctx, string outPath)
    {
        var factions = FactionAdapter.Read(ctx);
        var wrapped = new { factions };
        var json = JsonConvert.SerializeObject(wrapped, Formatting.Indented);
        CommandOutput.Write(outPath, json);
        return ExitCodes.Ok;
    }
}
