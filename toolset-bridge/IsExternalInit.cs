// Polyfill — net48 lacks IsExternalInit which records/init-only properties require.
namespace System.Runtime.CompilerServices;

internal static class IsExternalInit { }
