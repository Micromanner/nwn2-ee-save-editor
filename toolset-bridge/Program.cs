using System;
using ToolsetBridge.Adapters;

namespace ToolsetBridge;

public static class Program
{
    public static int Main(string[] args)
    {
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
