using System.Runtime.InteropServices;
using System.Text;

namespace RandomMac.App.Helpers;

/// <summary>
/// Thin wrapper around comdlg32 GetOpenFileName / GetSaveFileName.
///
/// Why not <c>Windows.Storage.Pickers.FileSavePicker</c>/<c>FileOpenPicker</c>?
/// In an unpackaged WinUI 3 process running with <c>requireAdministrator</c>,
/// the WinAppSDK pickers route through a broker that can't elevate
/// cross-process. The dialog either fails to open, returns null, or
/// throws "The parameter is incorrect" with no actionable diagnostic.
/// The legacy comdlg32 APIs run in-process and behave reliably under
/// elevation. Microsoft's own guidance for elevated WinUI 3 unpackaged
/// apps is to fall back to Win32 file dialogs.
/// </summary>
public static class Win32FileDialog
{
    /// <summary>
    /// Show the system "Save As" dialog. Returns the chosen path, or
    /// <c>null</c> if the user cancelled.
    /// </summary>
    public static string? PickSave(
        IntPtr ownerHwnd,
        string title,
        string suggestedFileName,
        string defaultExt,
        params (string Label, string Pattern)[] filters)
    {
        return Show(
            ownerHwnd,
            title,
            suggestedFileName,
            defaultExt,
            filters,
            OFN_OVERWRITEPROMPT | OFN_HIDEREADONLY | OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST,
            isSave: true);
    }

    /// <summary>
    /// Show the system "Open" dialog. Returns the chosen path, or
    /// <c>null</c> if the user cancelled.
    /// </summary>
    public static string? PickOpen(
        IntPtr ownerHwnd,
        string title,
        string defaultExt,
        params (string Label, string Pattern)[] filters)
    {
        return Show(
            ownerHwnd,
            title,
            "",
            defaultExt,
            filters,
            OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY | OFN_EXPLORER | OFN_NOCHANGEDIR,
            isSave: false);
    }

    private static string? Show(
        IntPtr ownerHwnd,
        string title,
        string suggestedFileName,
        string defaultExt,
        (string Label, string Pattern)[] filters,
        uint flags,
        bool isSave)
    {
        const int BufferLength = 2048;
        var buffer = new StringBuilder(BufferLength);
        if (!string.IsNullOrEmpty(suggestedFileName))
            buffer.Append(suggestedFileName);

        // Filter string layout: "Label1\0Pattern1\0Label2\0Pattern2\0\0"
        var filterSb = new StringBuilder();
        foreach (var f in filters)
        {
            filterSb.Append(f.Label).Append('\0').Append(f.Pattern).Append('\0');
        }
        filterSb.Append('\0');

        var ofn = new OpenFileNameW
        {
            structSize = Marshal.SizeOf<OpenFileNameW>(),
            hwnd = ownerHwnd,
            filter = filterSb.ToString(),
            file = buffer,
            maxFile = BufferLength,
            title = title,
            defExt = (defaultExt ?? "").TrimStart('.'),
            flags = flags,
        };

        var ok = isSave ? GetSaveFileNameW(ofn) : GetOpenFileNameW(ofn);
        return ok ? buffer.ToString() : null;
    }

    // OFN_* flags from commdlg.h
    private const uint OFN_HIDEREADONLY    = 0x00000004;
    private const uint OFN_OVERWRITEPROMPT = 0x00000002;
    private const uint OFN_NOCHANGEDIR     = 0x00000008;
    private const uint OFN_PATHMUSTEXIST   = 0x00000800;
    private const uint OFN_FILEMUSTEXIST   = 0x00001000;
    private const uint OFN_EXPLORER        = 0x00080000;

    [DllImport("comdlg32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool GetOpenFileNameW([In, Out] OpenFileNameW ofn);

    [DllImport("comdlg32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool GetSaveFileNameW([In, Out] OpenFileNameW ofn);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private sealed class OpenFileNameW
    {
        public int structSize;
        public IntPtr hwnd;
        public IntPtr hInstance;
        public string? filter;
        public string? customFilter;
        public int maxCustomFilter;
        public int filterIndex;
        public StringBuilder file = new();
        public int maxFile;
        public StringBuilder? fileTitle;
        public int maxFileTitle;
        public string? initialDir;
        public string? title;
        public uint flags;
        public short fileOffset;
        public short fileExtension;
        public string? defExt;
        public IntPtr custData;
        public IntPtr hook;
        public string? templateName;
        public IntPtr pvReserved;
        public int dwReserved;
        public int FlagsEx;
    }
}
