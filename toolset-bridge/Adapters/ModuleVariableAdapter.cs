using System.Collections.Generic;
using ToolsetBridge.Schema;

namespace ToolsetBridge.Adapters;

public static class ModuleVariableAdapter
{
    public static List<ModuleVariableShape> Read(LoadedContext ctx)
    {
        var result = new List<ModuleVariableShape>();
        if (ctx.Module?.ModuleInfo?.Variables == null) return result;

        foreach (var obj in ctx.Module.ModuleInfo.Variables)
        {
            if (obj is not NWN2Toolset.NWN2.Data.NWN2ScriptVariable v) continue;

            result.Add(new ModuleVariableShape
            {
                Name = v.Name ?? "",
                Type = NormalizeType(v.VariableType.ToString()),
                Default = ExtractDefault(v),
                Storage = "module_ifo_vartable",
            });
        }

        return result;
    }

    private static string NormalizeType(string variableType) =>
        variableType.ToLowerInvariant() switch
        {
            "int" => "int",
            "float" => "float",
            "string" => "string",
            "unsigned" => "int",
            _ => variableType.ToLowerInvariant(),
        };

    private static object? ExtractDefault(NWN2Toolset.NWN2.Data.NWN2ScriptVariable v) =>
        v.VariableType.ToString().ToLowerInvariant() switch
        {
            "int" => v.ValueInt,
            "float" => v.ValueFloat,
            "string" => v.ValueString,
            "unsigned" => (int)v.ValueUnsigned,
            _ => v.Value,
        };
}
