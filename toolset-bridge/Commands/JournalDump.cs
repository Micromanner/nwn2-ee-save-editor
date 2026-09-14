using System.IO;
using ToolsetBridge.Adapters;

namespace ToolsetBridge.Commands;

public static class JournalDump
{
    public static int Run(LoadedContext ctx, string outPath)
    {
        var shape = JournalAdapter.Read(ctx);
        var wrapped = new { journal = shape };
        var json = Json.Serialize(wrapped, indented: true);
        CommandOutput.Write(outPath, json);
        return ExitCodes.Ok;
    }
}
