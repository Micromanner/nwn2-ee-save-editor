using System;
using System.IO;

namespace ToolsetBridge.Commands;

internal static class CommandOutput
{
    public static void Write(string outPath, string json)
    {
        if (outPath == "-") Console.Out.WriteLine(json);
        else File.WriteAllText(outPath, json);
    }
}
