using System;
using System.Collections.Generic;

namespace ToolsetBridge;

public static class FunctorClassifier
{
    // Update these tables with the verified wrapper names from toolset-bridge/README.md.
    private static readonly HashSet<string> JournalScripts = new(StringComparer.OrdinalIgnoreCase)
    {
        "ga_journal",
        "ga_journal_entry",
        "gc_check_journal_entry",
        "gc_journal_entry",
    };

    private static readonly HashSet<string> GlobalIntScripts = new(StringComparer.OrdinalIgnoreCase)
    {
        "ga_global_int", "gc_global_int", "gc_check_global_int",
    };

    private static readonly HashSet<string> GlobalStringScripts = new(StringComparer.OrdinalIgnoreCase)
    {
        "ga_global_string", "gc_global_string", "gc_check_global_string",
    };

    private static readonly HashSet<string> GlobalFloatScripts = new(StringComparer.OrdinalIgnoreCase)
    {
        "ga_global_float", "gc_global_float", "gc_check_global_float",
    };

    private static readonly HashSet<string> GlobalBoolScripts = new(StringComparer.OrdinalIgnoreCase)
    {
        "ga_global_bool", "gc_global_bool", "gc_check_global_bool",
    };

    private static readonly HashSet<string> ModuleLocalScripts = new(StringComparer.OrdinalIgnoreCase)
    {
        "ga_local_int", "gc_local_int",
        "ga_module_int", "gc_module_int",
    };

    public static string Classify(string script)
    {
        if (string.IsNullOrEmpty(script)) return "custom";
        if (JournalScripts.Contains(script)) return "journal";
        if (GlobalIntScripts.Contains(script)) return "global_int";
        if (GlobalStringScripts.Contains(script)) return "global_string";
        if (GlobalFloatScripts.Contains(script)) return "global_float";
        if (GlobalBoolScripts.Contains(script)) return "global_bool";
        if (ModuleLocalScripts.Contains(script)) return "module_local";
        return "custom";
    }
}
