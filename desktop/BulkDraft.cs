namespace RoLauncher.Desktop;

public sealed class BulkDraft
{
    public EditorDraft Fields { get; set; } = new();
    public bool ApplyResource { get; set; }
    public string ResourceMode { get; set; } = "normal";
    public string Affinity { get; set; } = "";
    public bool ApplyAntiAfk { get; set; }
    public bool AntiAfkEnabled { get; set; }
    public bool HeadlessForegroundActivity { get; set; }
    public string AntiAfkAction { get; set; } = "space";
    public bool ApplyAlias { get; set; }
    public bool ApplyTarget { get; set; }
    public bool ClearTarget { get; set; }
    public string? GameProfileId { get; set; }
    public bool ApplyRejoin { get; set; }
    public bool Rejoin { get; set; }
    public bool ApplyGroup { get; set; }
    public string Group { get; set; } = "";
    public bool ApplyFallback { get; set; }
    public string Fallback { get; set; } = "allow_public";
    public bool TryPatch(out Dictionary<string, object?> patch, out string error)
    {
        patch = []; error = "";
        if (ApplyAlias)
        {
            if (string.IsNullOrWhiteSpace(Fields.Alias) || Fields.Alias.Length > 100 || Fields.Alias.Any(char.IsControl))
            { error = "Enter an alias or pattern, up to 100 characters."; return false; }
            patch["alias"] = Fields.Alias;
        }
        if (ApplyTarget)
        {
            if (ClearTarget) patch["clear_target"] = true;
            else
            {
                if (!Fields.TryTarget(out var target, out error)) return false;
                patch["target"] = target;
                if (GameProfileId is not null) patch["game_profile_id"] = GameProfileId;
            }
        }
        if (ApplyRejoin) patch["auto_recovery"] = Rejoin;
        if (ApplyGroup)
        {
            if (Group.Length > 100 || Group.Any(char.IsControl)) { error = "Group: max 100 characters; no control characters."; return false; }
            patch["group"] = Group;
        }
        if (ApplyFallback) patch["fallback_policy"] = Fallback;
        if (ApplyResource)
        {
            if (!ResourceSettings.TryCreate(ResourceMode, Affinity, out var resource, out error)) return false;
            patch["resource"] = resource;
        }
        if (ApplyAntiAfk)
        {
            if (AntiAfkAction is not ("space" or "camera")) { error = "Choose Space or Camera."; return false; }
            patch["anti_afk"] = new AntiAfkSettings(AntiAfkEnabled, AntiAfkAction, HeadlessForegroundActivity);
        }
        if (patch.Count == 0) { error = "Check at least one Apply setting."; return false; }
        return true;
    }
}
