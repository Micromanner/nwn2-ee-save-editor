using System;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace ToolsetBridge;

/// Minimal x64 entry-point detour. .NET Framework 4.8 x64 has no tiered JIT, so the
/// native address returned after PrepareMethod is final; overwriting its first bytes
/// with an absolute jump redirects every call. The original can never be called again,
/// so the replacement must be self-contained and match the target's signature exactly
/// (instance target = static replacement taking `this` first).
internal static class Detour
{
    private const int StubLength = 12;
    private const uint PageExecuteReadWrite = 0x40;

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool VirtualProtect(IntPtr address, UIntPtr size, uint newProtect, out uint oldProtect);

    [DllImport("kernel32.dll")]
    private static extern IntPtr GetCurrentProcess();

    [DllImport("kernel32.dll")]
    private static extern bool FlushInstructionCache(IntPtr process, IntPtr address, UIntPtr size);

    public static void Install(MethodInfo target, MethodInfo replacement)
    {
        RuntimeHelpers.PrepareMethod(target.MethodHandle);
        RuntimeHelpers.PrepareMethod(replacement.MethodHandle);

        var entry = target.MethodHandle.GetFunctionPointer();
        var stub = new byte[StubLength];
        stub[0] = 0x48; // mov rax, imm64
        stub[1] = 0xB8;
        BitConverter.GetBytes(replacement.MethodHandle.GetFunctionPointer().ToInt64()).CopyTo(stub, 2);
        stub[10] = 0xFF; // jmp rax
        stub[11] = 0xE0;

        if (!VirtualProtect(entry, (UIntPtr)StubLength, PageExecuteReadWrite, out var oldProtect))
            throw new InvalidOperationException($"VirtualProtect failed: {Marshal.GetLastWin32Error()}");
        Marshal.Copy(stub, 0, entry, StubLength);
        VirtualProtect(entry, (UIntPtr)StubLength, oldProtect, out _);
        FlushInstructionCache(GetCurrentProcess(), entry, (UIntPtr)StubLength);

        var written = new byte[StubLength];
        Marshal.Copy(entry, written, 0, StubLength);
        for (var i = 0; i < StubLength; i++)
        {
            if (written[i] != stub[i])
                throw new InvalidOperationException($"detour write did not land for {target.DeclaringType?.Name}.{target.Name}");
        }
    }
}
