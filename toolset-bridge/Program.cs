using System;
using System.IO;
using System.Text;
using ToolsetBridge.Adapters;

namespace ToolsetBridge;

public static class Program
{
    public static int Main(string[] args)
    {
        // Force UTF-8 on stdout so graph JSON with non-ASCII bytes (e.g. localized
        // dialog strings in Neverwinter_A1) survives the pipe into the Rust client.
        // Without this, Windows transcodes through the active codepage and corrupts
        // bytes, making graph() fail with "stdout was not UTF-8" for affected modules.
        Console.OutputEncoding = new UTF8Encoding(false);

        Timing.Start();

        ParsedArgs parsed;
        try
        {
            parsed = ArgParser.Parse(args);
        }
        catch (ArgParseException ex)
        {
            Log.Error(ex.Message);
            return ExitCodes.ArgError;
        }

        try
        {
            Timing.Measure("resolver", () => AssemblyResolver.Install(parsed.Nwn2Install));
        }
        catch (Exception ex)
        {
            Log.Error($"resolver install failed: {ex.Message}");
            return ExitCodes.DllResolutionFailed;
        }

        // list-modules only parses campaign.cam — pure GFF, no toolset load chain.
        if (ArgParser.NoToolsetInitSubcommands.Contains(parsed.Subcommand))
        {
            try
            {
                var code = parsed.Subcommand.ToLowerInvariant() switch
                {
                    "list-modules" => Commands.ListModules.Run(parsed.CampaignPath!, parsed.Nwn2Install, parsed.OutPath),
                    _ => UnknownSubcommand(parsed.Subcommand),
                };
                Timing.Emit($"subcommand={parsed.Subcommand}");
                return code;
            }
            catch (Exception ex)
            {
                Log.Error($"adapter failed: {ex}");
                return ExitCodes.AdapterFailed;
            }
        }

        // Serve mode: pay resource_mgr init once, then loop on stdin requests.
        // Module loads happen per-request inside the serve loop (not here).
        if (ArgParser.ServeSubcommands.Contains(parsed.Subcommand))
        {
            try
            {
                Timing.Measure("resource_mgr", () => ToolsetLoader.EnsureResourceManager(parsed.Nwn2Install));
            }
            catch (Exception ex)
            {
                Log.Error($"resource_mgr init failed: {ex}");
                return ExitCodes.LoadFailed;
            }
            Timing.Emit("subcommand=serve phase=init");
            return Commands.Serve.Run(parsed.Nwn2Install);
        }

        LoadedContext ctx;
        try
        {
            Timing.Measure("resource_mgr", () => ToolsetLoader.EnsureResourceManager(parsed.Nwn2Install));
            var parsedCopy = parsed;
            ctx = Timing.Measure("module_open", () => parsedCopy.ModulePath != null
                ? ToolsetLoader.LoadModule(parsedCopy.ModulePath)
                : ToolsetLoader.LoadCampaign(parsedCopy.CampaignPath!));
        }
        catch (Exception ex)
        {
            Log.Error($"load failed: {ex}");
            return ExitCodes.LoadFailed;
        }

        Log.Info($"loaded: name={ctx.Name} haks={string.Join(",", ctx.Haks)}");

        try
        {
            var code = parsed.Subcommand.ToLowerInvariant() switch
            {
                "journal" => Commands.JournalDump.Run(ctx, parsed.OutPath),
                "faction" => Commands.FactionDump.Run(ctx, parsed.OutPath),
                "module" => Commands.ModuleVars.Run(ctx, parsed.OutPath),
                "convo" => Commands.ConvoScan.Run(ctx, parsed.OutPath),
                "graph" => Commands.Graph.Run(ctx, parsed.OutPath),
                _ => UnknownSubcommand(parsed.Subcommand),
            };
            Timing.Emit($"subcommand={parsed.Subcommand} module={Path.GetFileName(parsed.ModulePath ?? parsed.CampaignPath ?? "")}");
            return code;
        }
        catch (Exception ex)
        {
            Log.Error($"adapter failed: {ex}");
            return ExitCodes.AdapterFailed;
        }
    }

    private static int UnknownSubcommand(string name)
    {
        Log.Error($"subcommand not dispatched: {name}");
        return ExitCodes.Unknown;
    }
}
