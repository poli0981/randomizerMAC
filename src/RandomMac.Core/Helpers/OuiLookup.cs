using RandomMac.Core.Models;

namespace RandomMac.Core.Helpers;

/// <summary>
/// Lightweight OUI (Organizationally Unique Identifier) lookup.
/// Maps the first 3 bytes of a MAC address to a vendor name.
/// </summary>
public static class OuiLookup
{
    /// <summary>
    /// Get the vendor name for a MAC address.
    /// Returns "Locally Administered" for LA-bit MACs, vendor name if known, or null.
    /// </summary>
    public static string? GetVendor(MacAddress mac)
    {
        if (mac == MacAddress.Empty) return null;
        if (mac.IsLocallyAdministered) return "Locally Administered";

        var prefix = $"{mac.Bytes[0]:X2}{mac.Bytes[1]:X2}{mac.Bytes[2]:X2}";
        return OuiPrefixes.GetValueOrDefault(prefix);
    }

    // ~70 most common OUI prefixes from IEEE registry
    private static readonly Dictionary<string, string> OuiPrefixes = new(StringComparer.OrdinalIgnoreCase)
    {
        // Intel
        ["8C8CAA"] = "Intel",
        ["A4C3F0"] = "Intel",
        ["3C6AA7"] = "Intel",
        ["48A472"] = "Intel",
        ["84C5A6"] = "Intel",
        ["F8E43B"] = "Intel",
        ["001B21"] = "Intel",
        ["0013E8"] = "Intel",
        ["002314"] = "Intel",
        ["A0369F"] = "Intel",
        ["AC7BA1"] = "Intel",

        // Realtek
        ["001A2B"] = "Realtek",
        ["00E04C"] = "Realtek",
        ["48E244"] = "Realtek",
        ["D8CBC8"] = "Realtek",
        ["525400"] = "Realtek",
        ["001CC0"] = "Realtek",

        // Broadcom
        ["001018"] = "Broadcom",
        ["006171"] = "Broadcom",
        ["D86C63"] = "Broadcom",

        // Qualcomm / Atheros
        ["000A35"] = "Qualcomm",
        ["24050F"] = "Qualcomm",
        ["9CEFD5"] = "Qualcomm",
        ["001632"] = "Qualcomm Atheros",
        ["00036F"] = "Qualcomm Atheros",

        // MediaTek
        ["000CE7"] = "MediaTek",
        ["7C1B94"] = "MediaTek",

        // TP-Link
        ["EC086B"] = "TP-Link",
        ["500700"] = "TP-Link",
        ["C025E9"] = "TP-Link",
        ["0023CD"] = "TP-Link",
        ["F4F26D"] = "TP-Link",

        // D-Link
        ["001CF0"] = "D-Link",
        ["0015E9"] = "D-Link",

        // Netgear
        ["C03F0E"] = "Netgear",
        ["A42B8C"] = "Netgear",

        // Cisco
        ["005056"] = "VMware",
        ["001217"] = "Cisco",
        ["000C29"] = "VMware",
        ["000569"] = "VMware",

        // Apple
        ["A860B6"] = "Apple",
        ["D4619D"] = "Apple",
        ["B065BD"] = "Apple",
        ["F0B479"] = "Apple",
        ["3CE072"] = "Apple",

        // Samsung
        ["A8F274"] = "Samsung",
        ["78D6F0"] = "Samsung",
        ["BC7285"] = "Samsung",

        // Microsoft / Hyper-V
        ["00155D"] = "Microsoft Hyper-V",
        ["0050F2"] = "Microsoft",
        ["282986"] = "Microsoft",

        // Dell
        ["D4BED9"] = "Dell",
        ["F8DB88"] = "Dell",
        ["18DB9B"] = "Dell",

        // HP / HPE
        ["D4C9EF"] = "HP",
        ["308D99"] = "HP",
        ["2C768A"] = "HP",

        // Lenovo
        ["E82A44"] = "Lenovo",
        ["F48E38"] = "Lenovo",
        ["28D244"] = "Lenovo",

        // ASUS
        ["001A92"] = "ASUS",
        ["485B39"] = "ASUS",
        ["04D4C4"] = "ASUS",

        // Huawei
        ["ACBD0B"] = "Huawei",
        ["CC96A0"] = "Huawei",
        ["48DB50"] = "Huawei",

        // Xiaomi
        ["286C07"] = "Xiaomi",
        ["9C9970"] = "Xiaomi",

        // Google
        ["F4F5D8"] = "Google",
        ["3C5AB4"] = "Google",
    };
}
