using System.IO;
using Newtonsoft.Json;
using ToolsetBridge.Adapters;

namespace ToolsetBridge.Commands;

public static class JournalDump
{
    public static int Run(LoadedContext ctx, string outPath)
    {
        var shape = JournalAdapter.Read(ctx);
        var wrapped = new { journal = shape };
        var json = JsonConvert.SerializeObject(wrapped, Formatting.Indented);
        CommandOutput.Write(outPath, json);
        return ExitCodes.Ok;
    }
}
