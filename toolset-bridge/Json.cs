using System;
using System.Collections;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Reflection;
using System.Text;

namespace ToolsetBridge;

[AttributeUsage(AttributeTargets.Property)]
public sealed class JsonNameAttribute : Attribute
{
    public string Name { get; }
    public JsonNameAttribute(string name) => Name = name;
}

/// Minimal JSON writer and request-line reader covering exactly what the bridge emits:
/// strings, bool, int/long, float/double, enumerables as arrays, and POCOs or anonymous
/// objects via public readable properties in declaration order (`[JsonName]` sets the
/// wire name). Output matches Newtonsoft.Json under its default settings, which the
/// Rust side was written against.
public static class Json
{
    private readonly struct Prop
    {
        public readonly PropertyInfo Info;
        public readonly string Name;
        public Prop(PropertyInfo info, string name) { Info = info; Name = name; }
    }

    private static readonly ConcurrentDictionary<Type, Prop[]> s_props = new();
    private static readonly char[] s_floatMarkers = { '.', 'e', 'E' };

    private static Prop[] PropertiesOf(Type type) => s_props.GetOrAdd(type, static t => t
        .GetProperties(BindingFlags.Public | BindingFlags.Instance)
        .Where(p => p.CanRead && p.GetIndexParameters().Length == 0)
        .OrderBy(p => p.MetadataToken)
        .Select(p => new Prop(p, p.GetCustomAttribute<JsonNameAttribute>()?.Name ?? p.Name))
        .ToArray());

    public static string Serialize(object? value, bool indented)
    {
        var sb = new StringBuilder();
        WriteValue(sb, value, indented, 0);
        return sb.ToString();
    }

    private static void WriteValue(StringBuilder sb, object? value, bool indented, int depth)
    {
        switch (value)
        {
            case null: sb.Append("null"); break;
            case string s: WriteString(sb, s); break;
            case bool b: sb.Append(b ? "true" : "false"); break;
            case int i: sb.Append(i.ToString(CultureInfo.InvariantCulture)); break;
            case long l: sb.Append(l.ToString(CultureInfo.InvariantCulture)); break;
            case float f: WriteFloat(sb, !float.IsNaN(f) && !float.IsInfinity(f), f.ToString("R", CultureInfo.InvariantCulture)); break;
            case double d: WriteFloat(sb, !double.IsNaN(d) && !double.IsInfinity(d), d.ToString("R", CultureInfo.InvariantCulture)); break;
            case IEnumerable e: WriteArray(sb, e, indented, depth); break;
            default: WriteObject(sb, value, indented, depth); break;
        }
    }

    private static void WriteFloat(StringBuilder sb, bool finite, string text)
    {
        if (!finite)
        {
            WriteString(sb, text);
            return;
        }
        sb.Append(text);
        if (text.IndexOfAny(s_floatMarkers) < 0) sb.Append(".0");
    }

    private static void WriteObject(StringBuilder sb, object value, bool indented, int depth)
    {
        var props = PropertiesOf(value.GetType());
        sb.Append('{');
        for (var i = 0; i < props.Length; i++)
        {
            if (i > 0) sb.Append(',');
            if (indented) NewLine(sb, depth + 1);
            WriteString(sb, props[i].Name);
            sb.Append(indented ? ": " : ":");
            WriteValue(sb, props[i].Info.GetValue(value), indented, depth + 1);
        }
        if (indented && props.Length > 0) NewLine(sb, depth);
        sb.Append('}');
    }

    private static void WriteArray(StringBuilder sb, IEnumerable items, bool indented, int depth)
    {
        sb.Append('[');
        var any = false;
        foreach (var item in items)
        {
            if (any) sb.Append(',');
            any = true;
            if (indented) NewLine(sb, depth + 1);
            WriteValue(sb, item, indented, depth + 1);
        }
        if (indented && any) NewLine(sb, depth);
        sb.Append(']');
    }

    private static void NewLine(StringBuilder sb, int depth) => sb.Append(Environment.NewLine).Append(' ', depth * 2);

    private static void WriteString(StringBuilder sb, string s)
    {
        sb.Append('"');
        foreach (var c in s)
        {
            switch (c)
            {
                case '"': sb.Append("\\\""); break;
                case '\\': sb.Append("\\\\"); break;
                case '\b': sb.Append("\\b"); break;
                case '\f': sb.Append("\\f"); break;
                case '\n': sb.Append("\\n"); break;
                case '\r': sb.Append("\\r"); break;
                case '\t': sb.Append("\\t"); break;
                default:
                    if (c < ' ') sb.Append("\\u").Append(((int)c).ToString("x4", CultureInfo.InvariantCulture));
                    else sb.Append(c);
                    break;
            }
        }
        sb.Append('"');
    }

    /// Reads one request line such as {"op":"graph","module":"C:\\x"} into a flat map.
    /// Non-string values are skipped; the bridge only needs string fields.
    public static Dictionary<string, string?> ParseFlatObject(string line)
    {
        var result = new Dictionary<string, string?>();
        var i = 0;
        SkipWs(line, ref i);
        if (i >= line.Length || line[i] != '{') throw new FormatException("request is not a JSON object");
        i++;
        while (true)
        {
            SkipWs(line, ref i);
            if (i >= line.Length) throw new FormatException("unterminated JSON object");
            if (line[i] == '}') return result;
            if (line[i] == ',') { i++; continue; }
            if (line[i] != '"') throw new FormatException($"unexpected '{line[i]}' at {i}");
            var key = ReadString(line, ref i);
            SkipWs(line, ref i);
            if (i >= line.Length || line[i] != ':') throw new FormatException($"expected ':' at {i}");
            i++;
            SkipWs(line, ref i);
            if (i < line.Length && line[i] == '"') result[key] = ReadString(line, ref i);
            else SkipValue(line, ref i);
        }
    }

    private static void SkipWs(string s, ref int i)
    {
        while (i < s.Length && char.IsWhiteSpace(s[i])) i++;
    }

    private static string ReadString(string s, ref int i)
    {
        i++;
        var sb = new StringBuilder();
        while (true)
        {
            if (i >= s.Length) throw new FormatException("unterminated string");
            var c = s[i++];
            if (c == '"') return sb.ToString();
            if (c != '\\')
            {
                sb.Append(c);
                continue;
            }
            if (i >= s.Length) throw new FormatException("unterminated escape");
            var e = s[i++];
            switch (e)
            {
                case 'b': sb.Append('\b'); break;
                case 'f': sb.Append('\f'); break;
                case 'n': sb.Append('\n'); break;
                case 'r': sb.Append('\r'); break;
                case 't': sb.Append('\t'); break;
                case 'u':
                    if (i + 4 > s.Length) throw new FormatException("truncated \\u escape");
                    sb.Append((char)int.Parse(s.Substring(i, 4), NumberStyles.HexNumber, CultureInfo.InvariantCulture));
                    i += 4;
                    break;
                case '"':
                case '\\':
                case '/':
                    sb.Append(e);
                    break;
                default: throw new FormatException($"bad JSON escape sequence: \\{e}");
            }
        }
    }

    private static void SkipValue(string s, ref int i)
    {
        var depth = 0;
        while (i < s.Length)
        {
            var c = s[i];
            if (c == '"') { ReadString(s, ref i); if (depth == 0) return; continue; }
            if (c == '{' || c == '[') depth++;
            else if (c == '}' || c == ']')
            {
                if (depth == 0) return;
                depth--;
                i++;
                if (depth == 0) return;
                continue;
            }
            else if (depth == 0 && c == ',') return;
            i++;
        }
    }
}
