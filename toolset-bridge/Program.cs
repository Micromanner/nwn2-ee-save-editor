using System;
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
            AssemblyResolver.Install(parsed.Nwn2Install);
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
                return parsed.Subcommand.ToLowerInvariant() switch
                {
                    "list-modules" => Commands.ListModules.Run(parsed.CampaignPath!, parsed.Nwn2Install, parsed.OutPath),
                    _ => UnknownSubcommand(parsed.Subcommand),
                };
            }
            catch (Exception ex)
            {
                Log.Error($"adapter failed: {ex}");
                return ExitCodes.AdapterFailed;
            }
        }

        LoadedContext ctx;
        try
        {
            ToolsetLoader.EnsureResourceManager(parsed.Nwn2Install);
            ctx = parsed.ModulePath != null
                ? ToolsetLoader.LoadModule(parsed.ModulePath)
                : ToolsetLoader.LoadCampaign(parsed.CampaignPath!);
        }
        catch (Exception ex)
        {
            Log.Error($"load failed: {ex}");
            return ExitCodes.LoadFailed;
        }

        Log.Info($"loaded: name={ctx.Name} haks={string.Join(",", ctx.Haks)}");

        try
        {
            return parsed.Subcommand.ToLowerInvariant() switch
            {
                "journal" => Commands.JournalDump.Run(ctx, parsed.OutPath),
                "faction" => Commands.FactionDump.Run(ctx, parsed.OutPath),
                "module" => Commands.ModuleVars.Run(ctx, parsed.OutPath),
                "convo" => Commands.ConvoScan.Run(ctx, parsed.OutPath),
                "graph" => Commands.Graph.Run(ctx, parsed.OutPath),
                _ => UnknownSubcommand(parsed.Subcommand),
            };
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
