using Newtonsoft.Json;
using ToolsetBridge.Adapters;

namespace ToolsetBridge.Commands;

public static class ModuleVars
{
    public static int Run(LoadedContext ctx, string outPath)
    {
        var vars = ModuleVariableAdapter.Read(ctx);
        var wrapped = new { module_variables = vars };
        var json = JsonConvert.SerializeObject(wrapped, Formatting.Indented);
        CommandOutput.Write(outPath, json);
        return ExitCodes.Ok;
    }
}
