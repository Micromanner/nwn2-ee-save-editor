using System;

namespace ToolsetBridge;

public static class Log
{
    public static void Info(string message) =>
        Console.Error.WriteLine($"[info] {message}");

    public static void Warn(string message) =>
        Console.Error.WriteLine($"[warn] {message}");

    public static void Error(string message) =>
        Console.Error.WriteLine($"[error] {message}");
}
