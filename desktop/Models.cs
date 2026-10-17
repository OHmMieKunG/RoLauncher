using System.ComponentModel;
using System.Text.Json.Serialization;

namespace RoLauncher.Desktop;

public sealed record Target(ulong PlaceId, string? JobId, string? PrivateServerLink);
public sealed record ProcessInfo(uint Pid);
public sealed record Account(string Id, string Username, string Alias, Target? Target,
    bool AutoRecovery, string Status, ProcessInfo? Process, uint Failures, string? LastError,
    string Group = "", string FallbackPolicy = "allow_public", DateTimeOffset? NextRetry = null,
    string RecoveryReason = "", bool PublicFallbackActive = false, bool DesiredRunning = false,
    DateTimeOffset? RunningSince = null, ulong LongestStreakSeconds = 0, string? GameProfileId = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ResourceSettings? Resource = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] AntiAfkSettings? AntiAfk = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ClientSettings? Client = null,
    DateTimeOffset? ConnectedSince = null, DateTimeOffset? DisconnectedSince = null, DateTimeOffset? LastDisconnect = null, ulong ReconnectionAttempts = 0)
{
    [System.Text.Json.Serialization.JsonIgnore]
    public ResourceSettings ResourceOptions => Resource ?? new();
    [System.Text.Json.Serialization.JsonIgnore]
    public AntiAfkSettings AntiAfkOptions => AntiAfk ?? new();
    [JsonIgnore]
    public ClientSettings ClientOptions => Client ?? new();
}
public sealed record ProfileEntry(string AccountId, string Alias, Target? Target, bool AutoRecovery, string FallbackPolicy, string Group, string? GameProfileId = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ResourceSettings? Resource = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] AntiAfkSettings? AntiAfk = null,
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] ClientSettings? Client = null);
public sealed record LaunchProfile(string Id, string Name, ProfileEntry[] Entries)
{
    public override string ToString() => $"{Name} · {Entries.Length} accounts";
}
public sealed record GameProfile(string Id, string Name, string GameName, string? ThumbnailUrl, Target Target)
{
    public string Destination => $"Place {Target.PlaceId} · {(Target.PrivateServerLink is not null ? "Private server" : Target.JobId is not null ? "Specific server" : "Public server")}";
    public override string ToString() => $"{Name} · {GameName}";
}
public sealed record Snapshot(Account[] Accounts, bool NetworkSuspended, string Compatibility, LaunchProfile[]? Profiles = null, string UpdateRepository = "", bool IncludeBetaUpdates = false, GameProfile[]? GameProfiles = null, bool CloseToTray = true, string ServerSelection = "smart", string JoinType = "random", Dictionary<string, RuntimeControls>? RuntimeControls = null, bool AntiAfkPaused = false, bool SessionSuspended = false, string NoRenderReason = "True no-render unavailable: live rendering validation pending", AggregateMetrics? Metrics = null, string NetworkOnlyReason = "Network-only unsupported: no documented player-session SDK");
public sealed record UpdateView(bool Available, string Version, string ReleaseUrl, string DownloadUrl, string ChecksumsUrl, string InstallerUrl = "", bool Prerelease = false);
public sealed record Activity(DateTimeOffset Timestamp, string AccountId, ulong? PlaceId, string Status, uint Failures, string Message)
{
    public string? AccountLabel { get; init; }
    public string Summary => $"{Timestamp.ToLocalTime():g} · {AccountLabel ?? (AccountId.Length == 0 ? "RoLauncher" : $"Account {AccountId}")} · {Status.Replace('_', ' ')} · {Message}";
}
public sealed record Notice(DateTimeOffset Timestamp, string Account, string Title, string Message, ulong? PlaceId);
public sealed record BotView(bool Enabled, bool Configured, string GuildId, string[] AllowedUsers, string Status, string ConnectionState = "offline");
public sealed record DiscordView(bool Enabled, bool Configured, bool NotifyRecovery, string DeliveryStatus, Notice[] Recent, BotView? Bot = null);

public sealed class AccountRow(Account account) : INotifyPropertyChanged
{
    public Account Account { get; private set; } = account;
    private RuntimeControls? _runtime;
    public string ResourceSummary => $"CPU {(_runtime?.CpuPercent is { } cpu ? $"{cpu:F1}%" : "unavailable")} | Working set {(_runtime?.WorkingSetBytes is { } memory ? $"{memory / 1048576.0:F1} MiB" : "unavailable")} | Private {(_runtime?.PrivateBytes is { } allocation ? $"{allocation / 1048576.0:F1} MiB" : "unavailable")} | Page faults {(_runtime?.PageFaultsPerSecond is { } faults ? $"{faults:F1}/s (includes soft faults)" : "unavailable")} | GPU {(_runtime?.GpuPercent is { } gpu ? $"{gpu:F1}% ({_runtime.GpuEngine})" : "unavailable")}";
    public string ControlSummary => _runtime is null ? "No verified process measurements" : $"{_runtime.ResourceResult}\n{_runtime.InputResult}\nActivity attempted: {_runtime.LastActivityAttemptAt?.ToLocalTime().ToString("T") ?? "never"} | OS input submitted: {_runtime.LastInputSubmittedAt?.ToLocalTime().ToString("T") ?? "never"}\nSample: {_runtime.SampledAt?.ToLocalTime():T}\n{string.Join("; ", _runtime.Unavailable ?? [])}";
    public string ClientSummary => $"{Account.ClientOptions.Bootstrapper} | requested {Account.ClientOptions.Mode} | {(_runtime?.ClientMode is { } mode ? $"observed {mode}" : "mode unverified")} | {Account.ResourceOptions.Mode}";
    public string SessionSummary => $"Window: {_runtime?.WindowState ?? "unavailable"} | Process: {(_runtime?.ProcessUptimeSeconds is { } seconds ? FormatDuration(seconds) : "unavailable")} | Connection: {(Account.Status == "running" && Account.ConnectedSince is { } since ? FormatDuration((ulong)Math.Max(0, (DateTimeOffset.UtcNow - since).TotalSeconds)) : "unverified")}\nRecovery attempts: {Account.ReconnectionAttempts} | Last disconnect: {Account.LastDisconnect?.ToLocalTime().ToString("g") ?? "none recorded"}";
    public void SetRuntime(RuntimeControls? runtime)
    {
        _runtime = runtime?.SampledAt is { } at && DateTimeOffset.UtcNow - at < TimeSpan.FromSeconds(6) ? runtime : null;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(ResourceSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(ControlSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(ClientSummary)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(SessionSummary)));
    }
    private bool _isSelected;
    public bool IsSelected { get => _isSelected; set { if (_isSelected == value) return; _isSelected = value; PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(IsSelected))); } }
    public string Id => Account.Id;
    public string Alias => Account.Alias;
    public string Group => string.IsNullOrEmpty(Account.Group) ? "Ungrouped" : Account.Group;
    public string Retry => Account.NextRetry is { } time ? $"Retry in {Math.Max(0, (int)(time - DateTimeOffset.UtcNow).TotalSeconds)}s · {time.ToLocalTime():T}" : Account.Status == "reconnecting" ? "Waiting up to 30 seconds to reconnect" : "No retry scheduled";
    public string Recovery => Account.RecoveryReason switch { "Auth" => "Session expired · sign in again", "Permission" => "Experience denied access", "TargetUnavailable" => "Destination unavailable", "Unsupported" => "Integration needs attention", "RateLimit" => "Rate limited · retry pending", "Network" => "Connection lost", _ => Error };
    public bool CanRetry => Account.Status == "backoff" && Account.NextRetry is not null && Account.AutoRecovery && Account.DesiredRunning && Account.RecoveryReason != "RateLimit";
    public string Fallback => Account.PublicFallbackActive ? "Public fallback active" : Account.FallbackPolicy switch { "stay" => "Stay on destination", "pause" => "Pause and notify", _ => "Public fallback allowed" };
    public string Username => $"@{Account.Username}";
    public ulong RunningSeconds => Account.RunningSince is { } since ? (ulong)Math.Max(0, (DateTimeOffset.UtcNow - since).TotalSeconds) : 0;
    public ulong LongestSeconds => Math.Max(Account.LongestStreakSeconds, RunningSeconds);
    public string RunningTime => Account.RunningSince is null ? "—" : FormatDuration(RunningSeconds);
    public string LongestOnline => FormatDuration(LongestSeconds);
    public static string FormatDuration(ulong seconds) => seconds >= 86400
        ? $"{seconds / 86400}d {seconds % 86400 / 3600}h {seconds % 3600 / 60}m"
        : seconds >= 3600 ? $"{seconds / 3600}h {seconds % 3600 / 60}m {seconds % 60}s"
        : seconds >= 60 ? $"{seconds / 60}m {seconds % 60}s" : $"{seconds}s";
    public string Pid => Account.Process?.Pid.ToString() ?? "—";
    public string Status => Account.Status switch
    {
        "needs_attention" => "Needs attention",
        "stopped" => "Stopped", "queued" => "Queued", "launching" => "Launching",
        "running" => "Running", "reconnecting" => "Reconnecting", "backoff" => "Backoff", _ => "Unknown"
    };
    private string? _gameName;
    public string Destination => Account.Target is null ? "Choose a game" : _gameName ?? $"Place {Account.Target.PlaceId}";
    public void SetGameName(string? name)
    {
        if (_gameName == name) return;
        _gameName = name;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Destination)));
    }
    public string DestinationKind => Account.Target?.PrivateServerLink is not null ? "Private server" : Account.Target?.JobId is not null ? "Specific server" : "Public server";
    public string Failures => Account.Failures.ToString();
    public string RecoveryAttempts => $"{Account.Failures}/5 consecutive failed attempts";
    public string Rejoin => Account.AutoRecovery ? "On" : "Off";
    public string Error => (Account.LastError ?? "—").Replace("recovery", "rejoin").Replace("Recovery", "Rejoin");
    public void Update(Account account)
    {
        if (Account.Target?.PlaceId != account.Target?.PlaceId) _gameName = null;
        Account = account;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(null));
    }
    public event PropertyChangedEventHandler? PropertyChanged;
}

public sealed record ResourceSettings(string Mode = "normal", string? Affinity = null)
{
    public static bool TryCreate(string mode, string affinity, out ResourceSettings settings, out string error)
    {
        settings = new(); error = "Choose Default, Balanced or Low Resource.";
        if (mode is not ("normal" or "balanced" or "low_resource")) return false;
        var text = affinity.Trim();
        var digits = text.StartsWith("0x", StringComparison.Ordinal) ? text[2..] : text;
        if (text.Length != 0 && (digits.Length is 0 or > 16 || !digits.All(char.IsAsciiHexDigit) ||
            !ulong.TryParse(digits, System.Globalization.NumberStyles.AllowHexSpecifier, System.Globalization.CultureInfo.InvariantCulture, out var mask) || mask == 0))
        { error = "Affinity must be a nonzero hexadecimal mask (for example 0x3), or blank."; return false; }
        settings = new(mode, text.Length == 0 ? null : text); error = ""; return true;
    }
}
public sealed record AntiAfkSettings(bool Enabled = false, string Action = "space", bool HeadlessForegroundActivity = false);
public sealed record RuntimeControls(ProcessInfo? Identity, DateTimeOffset? SampledAt, double? CpuPercent, ulong? WorkingSetBytes,
    double? GpuPercent, string? GpuEngine, string[]? Unavailable, string ResourceResult, string InputResult, string RobloxAcceptance,
    string? ClientMode = null, string WindowState = "unavailable", ulong? ProcessUptimeSeconds = null,
    ulong? PrivateBytes = null, double? PageFaultsPerSecond = null,
    DateTimeOffset? LastActivityAttemptAt = null, DateTimeOffset? LastInputSubmittedAt = null);
public sealed record ClientSettings(string Mode = "normal", string Bootstrapper = "system", string? Executable = null,
    bool AutoHide = false, bool StartOnOpen = false, bool Monitoring = true);
public sealed record BootstrapperInstallation(string Provider, string? Executable, string? Configuration, bool Available, string Reason)
{
    [JsonIgnore]
    public string Name => Provider switch { "system" => "Auto-detect", "official" => "Official Roblox", "bloxstrap" => "Bloxstrap", "fishstrap" => "Fishstrap", _ => "Custom" };
    [JsonIgnore]
    public string AutoLabel => !Available || Executable is null ? "Auto-detect (unavailable)" :
        $"Auto-detect ({Path.GetFileName(Executable.Replace('\\', '/')).ToLowerInvariant() switch { "fishstrap.exe" => "Fishstrap", "bloxstrap.exe" => "Bloxstrap", "robloxplayerbeta.exe" => "Official Roblox", _ => "registered launcher" }})";
    [JsonIgnore]
    public string DetectionLabel => Executable is null ? $"{Name}: not detected — {Reason}" :
        $"{Name}: detected{(Available ? " (ready)" : $" — {Reason}")}";
}
public sealed record AggregateMetrics(int ActiveAccounts, int ConnectedClients, double? CpuPercent, ulong? WorkingSetBytes, int MeasuredClients, double? GpuPercent);
