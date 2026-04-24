using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;

namespace ToolsetBridge;

/// Per-spawn phase timings. One emission to stderr at end of Main as a single
/// `[timing]` line; the Rust client forwards it through tracing so per-module
/// cost breakdowns land in the dev log.
public static class Timing
{
    private static readonly Stopwatch s_total = new();
    private static readonly List<(string Name, long Ms)> s_phases = new();

    public static void Start()
    {
        s_phases.Clear();
        s_total.Restart();
    }

    public static void Measure(string name, Action action)
    {
        var sw = Stopwatch.StartNew();
        try { action(); }
        finally { s_phases.Add((name, sw.ElapsedMilliseconds)); }
    }

    public static T Measure<T>(string name, Func<T> func)
    {
        var sw = Stopwatch.StartNew();
        try { return func(); }
        finally { s_phases.Add((name, sw.ElapsedMilliseconds)); }
    }

    public static void Emit(string context)
    {
        var phases = string.Join(" ", s_phases.Select(p => $"{p.Name}={p.Ms}ms"));
        Console.Error.WriteLine($"[timing] total={s_total.ElapsedMilliseconds}ms {phases} context=\"{context}\"");
    }
}
