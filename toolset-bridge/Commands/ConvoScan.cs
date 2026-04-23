using Newtonsoft.Json;
using ToolsetBridge.Adapters;

namespace ToolsetBridge.Commands;

public static class ConvoScan
{
    public static int Run(LoadedContext ctx, string outPath)
    {
        var shape = ConversationAdapter.Read(ctx);
        var wrapped = new { convo = shape };
        var json = JsonConvert.SerializeObject(wrapped, Formatting.Indented);
        CommandOutput.Write(outPath, json);
        return ExitCodes.Ok;
    }
}
