using ToolsetBridge.Adapters;

namespace ToolsetBridge.Commands;

public static class FactionDump
{
    public static int Run(LoadedContext ctx, string outPath)
    {
        var factions = FactionAdapter.Read(ctx);
        var wrapped = new { factions };
        var json = Json.Serialize(wrapped, indented: true);
        CommandOutput.Write(outPath, json);
        return ExitCodes.Ok;
    }
}
