using System;
using System.Collections.Generic;

namespace ToolsetBridge;

public sealed class ArgParseException : Exception
{
    public ArgParseException(string message) : base(message) { }
}

public sealed record ParsedArgs(
    string Subcommand,
    string? SubAction,
    string Nwn2Install,
    string? ModulePath,
    string? CampaignPath,
    string OutPath);

public static class ArgParser
{
    private static readonly HashSet<string> KnownSubcommands =
        new(StringComparer.OrdinalIgnoreCase) { "journal", "faction", "module", "convo", "graph", "list-modules", "serve" };

    // Subcommands that skip the heavy toolset init (ResourceManager, TLK, Harmony patches).
    // They still require --nwn2-install because OEIShared.dll hosts the GFF reader they use.
    public static readonly HashSet<string> NoToolsetInitSubcommands =
        new(StringComparer.OrdinalIgnoreCase) { "list-modules" };

    public static readonly HashSet<string> ServeSubcommands =
        new(StringComparer.OrdinalIgnoreCase) { "serve" };

    public static ParsedArgs Parse(string[] args)
    {
        string? nwn2Install = null;
        string? modulePath = null;
        string? campaignPath = null;
        string outPath = "-";

        var positionals = new List<string>();
        for (int i = 0; i < args.Length; i++)
        {
            var a = args[i];
            switch (a)
            {
                case "--nwn2-install":
                    nwn2Install = RequireNext(args, ref i, a);
                    break;
                case "--module":
                    modulePath = RequireNext(args, ref i, a);
                    break;
                case "--campaign":
                    campaignPath = RequireNext(args, ref i, a);
                    break;
                case "--out":
                    outPath = RequireNext(args, ref i, a);
                    break;
                default:
                    if (a.StartsWith("--"))
                        throw new ArgParseException($"Unknown flag: {a}");
                    positionals.Add(a);
                    break;
            }
        }

        if (positionals.Count == 0)
            throw new ArgParseException("Missing subcommand");
        var subcommand = positionals[0];
        if (!KnownSubcommands.Contains(subcommand))
            throw new ArgParseException($"Unknown subcommand: {subcommand}");

        string? subAction = null;
        if (!subcommand.Equals("graph", StringComparison.OrdinalIgnoreCase) && positionals.Count >= 2)
            subAction = positionals[1];

        if (string.IsNullOrWhiteSpace(nwn2Install))
            throw new ArgParseException("--nwn2-install is required");

        if (subcommand.Equals("list-modules", StringComparison.OrdinalIgnoreCase))
        {
            if (campaignPath == null)
                throw new ArgParseException("list-modules requires --campaign <path>");
            if (modulePath != null)
                throw new ArgParseException("list-modules does not accept --module");
        }
        else if (ServeSubcommands.Contains(subcommand))
        {
            // serve reads module/campaign paths from its stdin request loop, not argv.
            if (modulePath != null || campaignPath != null)
                throw new ArgParseException($"{subcommand} does not accept --module or --campaign");
        }
        else
        {
            if (modulePath == null && campaignPath == null)
                throw new ArgParseException("one of --module or --campaign is required");
            if (modulePath != null && campaignPath != null)
                throw new ArgParseException("--module and --campaign are mutually exclusive");
        }

        return new ParsedArgs(subcommand, subAction, nwn2Install ?? "", modulePath, campaignPath, outPath);
    }

    private static string RequireNext(string[] args, ref int i, string flag)
    {
        if (i + 1 >= args.Length)
            throw new ArgParseException($"{flag} requires a value");
        return args[++i];
    }
}
