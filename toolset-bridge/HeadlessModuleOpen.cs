using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using NWN2Toolset.NWN2.Data;
using NWN2Toolset.NWN2.IO;
using OEIShared.IO;
using OEIShared.IO.ERF;

namespace ToolsetBridge;

/// GUI-free equivalent of NWN2GameModule.OpenModuleFile / OpenModuleDirectory.
/// Both toolset methods construct the MODFile, then call OEIUnserialize(string), whose
/// last line is NWN2ToolsetMainForm.App.ContentManager.InitializeModule (null headlessly),
/// then InitJournalAndFactionData. This mirrors that sequence minus:
///   - MODFile: it unpacks the whole .mod (areas included, 0.15-0.43 GB) into the CWD and
///     its Close() shows a MessageBox on failure, which would hang the hidden process.
///     A .mod is instead extracted partially into the system temp folder, see Extract.
///   - LoadBlueprints (not needed by the bridge)
///   - ContentManager.InitializeModule (requires NWN2ToolsetMainForm.App)
///   - the ModuleExpanded event (no subscribers in the bridge process)
internal static class HeadlessModuleOpen
{
    private const BindingFlags Instance = BindingFlags.NonPublic | BindingFlags.Instance;

    /// Everything the adapters read: module.ifo, module.jrl, repute.fac, conversations.
    private static readonly string[] GraphExtensions = { "ifo", "jrl", "fac", "dlg" };

    private static readonly string ScratchRoot = Path.Combine(Path.GetTempPath(), "nwn2ee-toolset-bridge");

    private static readonly (int Id, string Name) Self = CurrentProcess();

    /// Opens the module and returns the handle that removes its resource repository and
    /// extracted files again.
    public static IDisposable Open(NWN2GameModule module, string path, bool isFile)
    {
        var t = typeof(NWN2GameModule);
        var fName = t.GetField("m_sName", Instance)!;
        var fFileName = t.GetField("m_sFileName", Instance)!;
        var fGRCData = t.GetField("m_cGRCData", Instance)!;
        var fModInfo = t.GetField("m_cModuleInfo", Instance)!;
        var fJournal = t.GetField("m_cJournal", Instance)!;
        var fFactionData = t.GetField("m_cFactionData", Instance)!;
        var mInitJournalAndFactionData = t.GetMethod("InitJournalAndFactionData", Instance)!;

        var extractedDir = isFile ? Extract(path) : null;
        var dirName = extractedDir ?? Path.GetFullPath(path);
        var opened = new OpenedModule(new DirectoryResourceRepository(dirName), extractedDir);
        try
        {
            NWN2ResourceManager.Instance.AddRepository(opened.Repository);

            fName.SetValue(module, Path.GetFileNameWithoutExtension(path));
            fFileName.SetValue(module, path);

            // Setting GRCData.Repository triggers PopulateContents, which fills the
            // Areas / Conversations / Scripts dictionaries the adapters read.
            var grcData = fGRCData.GetValue(module)!;
            grcData.GetType().GetProperty("Repository")!.SetValue(grcData, opened.Repository);

            Unserialize(fModInfo.GetValue(module)!, Path.Combine(dirName, "module.ifo"));

            var journalPath = Path.Combine(dirName, "module.jrl");
            if (File.Exists(journalPath))
                Unserialize(fJournal.GetValue(module)!, journalPath);

            var repute = Path.Combine(dirName, "repute.fac");
            if (File.Exists(repute))
                Unserialize(fFactionData.GetValue(module)!, repute);

            mInitJournalAndFactionData.Invoke(module, Array.Empty<object>());
            return opened;
        }
        catch
        {
            opened.Dispose();
            throw;
        }
    }

    /// Deletes extraction folders of bridge processes that are no longer running
    /// (killed before their request finished).
    public static void SweepStaleExtractions()
    {
        if (!Directory.Exists(ScratchRoot)) return;

        foreach (var dir in Directory.GetDirectories(ScratchRoot))
        {
            var pidText = Path.GetFileName(dir).Split('_')[0];
            if (int.TryParse(pidText, out var pid) && !IsBridgeProcess(pid))
                DeleteDirectory(dir);
        }
    }

    private static (int, string) CurrentProcess()
    {
        using var process = Process.GetCurrentProcess();
        return (process.Id, process.ProcessName);
    }

    private static bool IsBridgeProcess(int pid)
    {
        try
        {
            using var process = Process.GetProcessById(pid);
            return process.ProcessName == Self.Name;
        }
        catch (ArgumentException)
        {
            return false;
        }
    }

    private static string Extract(string modPath)
    {
        if (!string.Equals(Path.GetExtension(modPath), ".mod", StringComparison.OrdinalIgnoreCase))
            throw new NotSupportedException($"unsupported module file (expected .mod): {modPath}");

        var dir = Path.Combine(ScratchRoot,
            $"{Self.Id}_{Guid.NewGuid():N}_{Path.GetFileNameWithoutExtension(modPath)}");
        Directory.CreateDirectory(dir);
        try
        {
            using var erf = new ERFFile(modPath, bDemandLoad: true);
            foreach (ERFResource resource in erf.Resources)
            {
                if (GraphExtensions.Contains(resource.Extension, StringComparer.OrdinalIgnoreCase))
                    resource.SaveData(Path.Combine(dir, resource.ResRef.Value + "." + resource.Extension));
            }
        }
        catch
        {
            DeleteDirectory(dir);
            throw;
        }
        return dir;
    }

    private static void DeleteDirectory(string dir)
    {
        try
        {
            Directory.Delete(dir, recursive: true);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            Log.Warn($"could not delete {dir}: {ex.Message}");
        }
    }

    private static void Unserialize(object target, string path) =>
        target.GetType().GetMethod("OEIUnserialize", new[] { typeof(string) })!
            .Invoke(target, new object[] { path });

    private sealed class OpenedModule : IDisposable
    {
        public DirectoryResourceRepository Repository { get; }
        private readonly string? _extractedDir;

        public OpenedModule(DirectoryResourceRepository repository, string? extractedDir)
        {
            Repository = repository;
            _extractedDir = extractedDir;
        }

        public void Dispose()
        {
            NWN2ResourceManager.Instance.RemoveRepository(Repository);
            Repository.Dispose();
            if (_extractedDir != null)
                DeleteDirectory(_extractedDir);
        }
    }
}
