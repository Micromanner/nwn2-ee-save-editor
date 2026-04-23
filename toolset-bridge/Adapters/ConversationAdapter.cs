using System.Collections.Generic;
using System.Linq;
using ToolsetBridge.Schema;

namespace ToolsetBridge.Adapters;

public static class ConversationAdapter
{
    public static ConvoShape Read(LoadedContext ctx)
    {
        var shape = new ConvoShape();
        if (ctx.Module?.Conversations == null) return shape;

        foreach (var kv in ctx.Module.Conversations)
        {
            // OEIDictionaryWithEvents yields DictionaryEntry; value is NWN2GameConversation.
            NWN2Toolset.NWN2.Data.NWN2GameConversation? dlg = null;
            if (kv is System.Collections.DictionaryEntry de && de.Value is NWN2Toolset.NWN2.Data.NWN2GameConversation d1)
                dlg = d1;
            else if (kv is NWN2Toolset.NWN2.Data.NWN2GameConversation d2)
                dlg = d2;

            if (dlg == null) continue;

            // Conversations are reference-counted. Demand() loads on first ref, Release() unloads.
            bool demanded = false;
            if (!dlg.Loaded)
            {
                try { dlg.Demand(); demanded = true; }
                catch (System.Exception ex) { Log.Warn($"skip conversation {dlg.Name}: {ex.Message}"); continue; }
            }

            foreach (var connObj in dlg.GetAllConnectors())
            {
                if (connObj is not NWN2Toolset.NWN2.Data.ConversationData.NWN2ConversationConnector conn)
                    continue;

                var node = new ConvoNode
                {
                    Dlg = dlg.Name ?? "",
                    Node = conn.ConnectorID?.ID ?? -1,
                    Speaker = conn.Line?.Speaker ?? "",
                    TextStrref = conn.Line?.Text?.StringRefValid == true
                        ? (int)conn.Line.Text.StringRef
                        : -1,
                    Actions = ToEntries(conn.Actions),
                    Conditions = ToEntries(conn.Conditions),
                };

                // Connector's direct Quest declaration — authoritative journal state.
                if (!string.IsNullOrEmpty(conn.Quest?.Quest))
                {
                    node.Actions.Add(new ConvoFunctorEntry
                    {
                        Kind = "journal",
                        Script = "__connector_quest_tuple__",
                        Params = new object[] { conn.Quest!.Quest!, (int)conn.Quest.Entry },
                    });
                }

                shape.Nodes.Add(node);
            }

            if (demanded)
            {
                try { dlg.Release(); } catch { /* ignore release errors */ }
            }
        }

        return shape;
    }

    private static List<ConvoFunctorEntry> ToEntries(System.Collections.IEnumerable? functors)
    {
        var result = new List<ConvoFunctorEntry>();
        if (functors == null) return result;

        foreach (var obj in functors)
        {
            if (obj is not NWN2Toolset.NWN2.Data.NWN2ScriptFunctor f) continue;
            var script = f.Script?.ResRef.Value ?? "";
            result.Add(new ConvoFunctorEntry
            {
                Kind = FunctorClassifier.Classify(script),
                Script = script,
                Params = ExtractParams(f.Parameters),
            });
        }
        return result;
    }

    private static object[] ExtractParams(OEIShared.Utils.NWN2ScriptParameterCollection? parameters)
    {
        if (parameters == null) return System.Array.Empty<object>();
        var list = new List<object>();
        foreach (var obj in parameters)
        {
            if (obj is not OEIShared.Utils.NWN2ScriptParameter p)
            {
                list.Add(obj?.ToString() ?? "");
                continue;
            }
            list.Add(p.ParameterType.ToString().ToLowerInvariant() switch
            {
                "int" => p.ValueInt,
                "float" => p.ValueFloat,
                "string" => p.ValueString ?? "",
                "tag" => p.ValueTag ?? "",
                _ => p.Value?.ToString() ?? "",
            });
        }
        return list.ToArray();
    }
}
