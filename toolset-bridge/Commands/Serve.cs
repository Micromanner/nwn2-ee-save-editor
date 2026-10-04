using System;
using System.IO;
using System.Text;
using ToolsetBridge.Adapters;

namespace ToolsetBridge.Commands;

/// Long-lived NDJSON request loop. Resource-manager init, the CustomTalkFile detour, and
/// TLK load are paid once in `Program.cs` before we start reading stdin; each
/// request re-uses the warm process, amortizing ~1s of setup across N modules.
///
/// Protocol: one request per line on stdin, one response per line on stdout.
/// Request shapes:
///   {"op":"graph","module":"<path>"}
///   {"op":"list_modules","campaign":"<path>"}
///   {"op":"shutdown"}
/// Response shapes:
///   {"ok":true,"data":<payload>}
///   {"ok":false,"error":"<message>"}
public static class Serve
{
    public static int Run(string nwn2Install)
    {
        // Keep stdin reads unbuffered-ish: Console.In is already line-buffered,
        // so ReadLine blocks until a newline arrives. Stdout auto-flushes per
        // WriteLine on Console.
        Log.Info("serve: ready");

        string? line;
        while ((line = Console.In.ReadLine()) != null)
        {
            if (line.Length == 0) continue;

            Timing.Start();
            try
            {
                var req = Json.ParseFlatObject(line);
                req.TryGetValue("op", out var op);
                if (string.IsNullOrEmpty(op))
                {
                    EmitError("request missing 'op' field");
                    continue;
                }

                switch (op)
                {
                    case "graph":
                        req.TryGetValue("module", out var module);
                        HandleGraph(module ?? "");
                        break;
                    case "list_modules":
                        req.TryGetValue("campaign", out var campaign);
                        HandleListModules(campaign ?? "", nwn2Install);
                        break;
                    case "shutdown":
                        Log.Info("serve: shutdown requested");
                        return ExitCodes.Ok;
                    default:
                        EmitError($"unknown op: {op}");
                        break;
                }
            }
            catch (Exception ex)
            {
                EmitError($"{ex.GetType().Name}: {ex.Message}");
            }
            finally
            {
                Timing.Emit("subcommand=serve");
            }
        }

        Log.Info("serve: stdin closed, exiting");
        return ExitCodes.Ok;
    }

    private static void HandleGraph(string modulePath)
    {
        if (string.IsNullOrEmpty(modulePath))
        {
            EmitError("graph request missing 'module' field");
            return;
        }

        LoadedContext ctx;
        try
        {
            ctx = Timing.Measure("module_open", () => ToolsetLoader.LoadModule(modulePath));
        }
        catch (Exception ex)
        {
            EmitError($"module_open failed: {ex.Message}");
            return;
        }

        using (ctx)
        {
            var output = Graph.Compute(ctx);
            EmitOkRaw(Timing.Measure("serialize", () => Json.Serialize(output, indented: false)));
        }
    }

    private static void HandleListModules(string campaignPath, string nwn2Install)
    {
        if (string.IsNullOrEmpty(campaignPath))
        {
            EmitError("list_modules request missing 'campaign' field");
            return;
        }

        var data = Timing.Measure("list_modules", () => ListModules.Compute(campaignPath, nwn2Install));
        if (data == null)
        {
            EmitError($"list_modules failed to parse campaign: {campaignPath}");
            return;
        }

        var json = Json.Serialize(data, indented: false);
        EmitOkRaw(json);
    }

    /// Emit a successful response. `dataJson` is pre-serialized compact JSON and
    /// spliced directly into the envelope to avoid a redundant deserialize+serialize
    /// round-trip on multi-MB graph payloads.
    private static void EmitOkRaw(string dataJson)
    {
        // Single Write to stream out the envelope with no stray newlines between parts.
        var sb = new StringBuilder("{\"ok\":true,\"data\":");
        sb.Append(dataJson);
        sb.Append('}');
        Console.Out.WriteLine(sb.ToString());
    }

    private static void EmitError(string message)
    {
        var envelope = new { ok = false, error = message };
        Console.Out.WriteLine(Json.Serialize(envelope, indented: false));
    }
}
