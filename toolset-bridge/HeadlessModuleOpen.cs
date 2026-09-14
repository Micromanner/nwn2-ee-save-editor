using System;
using System.IO;
using System.Reflection;
using NWN2Toolset.NWN2.Data;
using NWN2Toolset.NWN2.IO;
using OEIShared.IO;

namespace ToolsetBridge;

/// GUI-free equivalent of NWN2GameModule.OpenModuleFile / OpenModuleDirectory.
/// Both toolset methods construct the MODFile, then call OEIUnserialize(string), whose
/// last line is NWN2ToolsetMainForm.App.ContentManager.InitializeModule (null headlessly),
/// then InitJournalAndFactionData. This mirrors that sequence minus:
///   - LoadBlueprints (not needed by the bridge)
///   - ContentManager.InitializeModule (requires NWN2ToolsetMainForm.App)
///   - the ModuleExpanded event (no subscribers in the bridge process)
internal static class HeadlessModuleOpen
{
    private const BindingFlags Instance = BindingFlags.NonPublic | BindingFlags.Instance;

    public static void Open(NWN2GameModule module, string path, bool isFile)
    {
        var t = typeof(NWN2GameModule);
        var fName = t.GetField("m_sName", Instance)!;
        var fFileName = t.GetField("m_sFileName", Instance)!;
        var fFile = t.GetField("m_cFile", Instance)!;
        var fGRCData = t.GetField("m_cGRCData", Instance)!;
        var fModInfo = t.GetField("m_cModuleInfo", Instance)!;
        var fJournal = t.GetField("m_cJournal", Instance)!;
        var fFactionData = t.GetField("m_cFactionData", Instance)!;
        var mInitJournalAndFactionData = t.GetMethod("InitJournalAndFactionData", Instance)!;

        var file = new MODFile(path, isFile ? ModuleLocationType.File : ModuleLocationType.Directory);
        fFile.SetValue(module, file);

        fName.SetValue(module, Path.GetFileNameWithoutExtension(path));
        fFileName.SetValue(module, path);
        file.Open(path);

        // Setting GRCData.Repository triggers PopulateContents, which fills the
        // Areas / Conversations / Scripts dictionaries the adapters read.
        var grcData = fGRCData.GetValue(module)!;
        grcData.GetType().GetProperty("Repository")!.SetValue(grcData, file.Repository as DirectoryResourceRepository);

        var dirName = file.DirectoryName;
        Unserialize(fModInfo.GetValue(module)!, Path.Combine(dirName, "module.ifo"));

        var journalPath = Path.Combine(dirName, "module.jrl");
        if (File.Exists(journalPath))
            Unserialize(fJournal.GetValue(module)!, journalPath);

        var repute = Path.Combine(dirName, "repute.fac");
        if (File.Exists(repute))
            Unserialize(fFactionData.GetValue(module)!, repute);

        mInitJournalAndFactionData.Invoke(module, Array.Empty<object>());
    }

    private static void Unserialize(object target, string path) =>
        target.GetType().GetMethod("OEIUnserialize", new[] { typeof(string) })!
            .Invoke(target, new object[] { path });
}
