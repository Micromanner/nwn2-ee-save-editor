using System;
using System.Collections.Generic;
using System.Text;
using System.Web.Script.Serialization;
using ToolsetBridge;

namespace ToolsetOracle;

/// Dev-only engine rules oracle. One request per process: `toolset-oracle --nwn2-install <dir> <op>`
/// reads the request JSON from stdin and writes `{"ok":true,"data":...}` or
/// `{"ok":false,"error":"..."}` to stdout. A fresh process per request keeps toolset
/// content (overrides, HAKs) from leaking between saves.
public static class Program
{
    public static int Main(string[] args)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        if (args.Length != 3 || args[0] != "--nwn2-install")
        {
            Log.Error("usage: toolset-oracle --nwn2-install <dir> <op>");
            return 2;
        }

        try
        {
            AssemblyResolver.Install(args[1]);
            var request = ReadRequest();
            Emit(new { ok = true, data = Run(args[1], args[2], request) });
            return 0;
        }
        catch (Exception ex)
        {
            Log.Error(ex.ToString());
            Emit(new { ok = false, error = $"{ex.GetType().Name}: {ex.Message}" });
            return 1;
        }
    }

    private static object Run(string install, string op, Dictionary<string, object> request)
    {
        var rules = RulesHost.Init(install, request);
        return op switch
        {
            "ping" => RulesHost.Info(rules),
            "build" => Build.Run(rules, request),
            "check" => Check.Run(rules, request),
            "level_up" => LevelUp.Run(rules, request),
            _ => throw new ArgumentException($"unknown op: {op}"),
        };
    }

    private static Dictionary<string, object> ReadRequest()
    {
        var text = Console.In.ReadToEnd();
        if (string.IsNullOrWhiteSpace(text)) return new Dictionary<string, object>();
        return new JavaScriptSerializer().Deserialize<Dictionary<string, object>>(text);
    }

    private static void Emit(object envelope) => Console.Out.WriteLine(Json.Serialize(envelope, indented: false));
}
