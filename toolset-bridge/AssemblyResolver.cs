using System;
using System.IO;
using System.Reflection;

namespace ToolsetBridge;

public static class AssemblyResolver
{
    private static string? _root;

    public static void Install(string nwn2InstallRoot)
    {
        _root = nwn2InstallRoot;
        AppDomain.CurrentDomain.AssemblyResolve += OnResolve;
    }

    private static Assembly? OnResolve(object? sender, ResolveEventArgs args)
    {
        if (_root == null) return null;
        var shortName = new AssemblyName(args.Name).Name;
        if (shortName == null) return null;
        var candidate = Path.Combine(_root, shortName + ".dll");
        if (File.Exists(candidate))
        {
            Log.Info($"resolving {shortName} from {candidate}");
            return Assembly.LoadFrom(candidate);
        }
        Log.Warn($"could not resolve {shortName} (tried {candidate})");
        return null;
    }
}
