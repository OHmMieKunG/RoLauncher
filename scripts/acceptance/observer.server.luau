-- Install through Studio in ServerScriptService of the owned test experience only.
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local DataStoreService = game:GetService("DataStoreService")
local HttpService = game:GetService("HttpService")

local runId = script:GetAttribute("RunId")
local placeId = script:GetAttribute("PlaceId")
local firstId = script:GetAttribute("FirstAccountId")
local secondId = script:GetAttribute("SecondAccountId")
assert(type(runId) == "string" and #runId <= 64 and runId:match("^[%w_-]+$"), "Set a unique RunId matching the harness output folder")
assert(placeId == game.PlaceId and firstId and secondId and firstId ~= secondId, "Configure the authorized place and two dedicated IDs")
local allowed = {[tostring(firstId)] = true, [tostring(secondId)] = true}
local store = DataStoreService:GetDataStore("RoLauncherAcceptanceV1")
local key = runId .. ":" .. game.JobId
local remote = Instance.new("RemoteEvent")
remote.Name = "RoLauncherAcceptanceHeartbeat"
remote.Parent = ReplicatedStorage
local records, active = {}, {}
local saving = false
local function now() return DateTime.now().UnixTimestampMillis end
local function save()
    if saving then return end
    saving = true
    local snapshot = HttpService:JSONDecode(HttpService:JSONEncode({schema = 1, run_id = runId,
        place_id = game.PlaceId, updated_ms = now(), records = records}))
    for attempt = 1, 3 do
        local ok = pcall(function()
            store:UpdateAsync(key, function(old)
                if old and old.updated_ms > snapshot.updated_ms then return old end
                return snapshot
            end)
        end)
        if ok then saving = false; return end
        task.wait(2 ^ attempt)
    end
    warn("RoLauncher observer save failed; export is incomplete")
    saving = false
end
local function joined(player)
    if not allowed[tostring(player.UserId)] or active[player] then return end
    if #records >= 64 then warn("RoLauncher observer connection bound reached"); return end
    local record = {account_id = tostring(player.UserId), nonce = HttpService:GenerateGUID(false),
        job_id = game.JobId, joined_ms = now(), observed_through_ms = now(), samples = {}}
    table.insert(records, record)
    active[player] = {record = record, progress = 0, acks = 0, lastAck = 0, inputs = 0, idles = 0}
    task.spawn(save)
end
Players.PlayerAdded:Connect(joined)
Players.PlayerRemoving:Connect(function(player)
    local state = active[player]
    if state then state.record.left_ms = now(); state.record.observed_through_ms = state.record.left_ms; active[player] = nil; task.spawn(save) end
end)
for _, player in Players:GetPlayers() do joined(player) end
remote.OnServerEvent:Connect(function(player, inputs, idles)
    local state = active[player]
    -- Rate-limit ordinary progress acknowledgements; never generate input or reset idle.
    if state and now() - state.lastAck >= 5000 then
        state.lastAck = now()
        state.acks += 1
        local function counter(value, previous)
            return type(value) == "number" and value == value and value % 1 == 0
                and value >= (previous or 0) and value <= 1000000
        end
        if counter(inputs, state.inputs) and counter(idles, state.idles) then
            state.inputs, state.idles = inputs, idles
        end
    end
end)
task.spawn(function()
    local ticks = 0
    while true do
        task.wait(10)
        for _, state in active do
            state.progress += 1
            state.record.observed_through_ms = now()
            if #state.record.samples < 720 then
                table.insert(state.record.samples, {at_ms = now(), progress_count = state.progress,
                    client_ack_count = state.acks, input_count = state.inputs, idle_event_count = state.idles})
            end
        end
        ticks += 1
        if ticks % 6 == 0 then save() end
    end
end)
game:BindToClose(function()
    local deadline = os.clock() + 20
    while saving and os.clock() < deadline do task.wait(.1) end
    for _, state in active do state.record.left_ms = now(); state.record.observed_through_ms = state.record.left_ms end
    save()
end)
print("RoLauncher observer export key: " .. key)
