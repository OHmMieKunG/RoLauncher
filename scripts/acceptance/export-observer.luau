-- Run in Studio's server Command Bar with API access enabled for the owned experience.
-- Add every runId:jobId printed by the observer, including replacement servers.
local keys = {"REPLACE_WITH_RUN_ID:JOB_ID"}
local store = game:GetService("DataStoreService"):GetDataStore("RoLauncherAcceptanceV1")
local combined = nil
for _, key in keys do
    local result = store:GetAsync(key)
    assert(result and result.schema == 1, "Observer record missing")
    if not combined then
        combined = {schema = 1, run_id = result.run_id, place_id = result.place_id, records = {}}
    end
    assert(combined.run_id == result.run_id and combined.place_id == result.place_id, "Do not combine different runs or places")
    for _, record in result.records do table.insert(combined.records, record) end
end
local value = Instance.new("StringValue")
value.Name = "RoLauncherAcceptanceExport"
value.Value = game:GetService("HttpService"):JSONEncode(combined)
value.Parent = game:GetService("ServerStorage")
game:GetService("Selection"):Set({value})
print("Copy the selected StringValue.Value to a local UTF-8 server-export.json; do not publish the export instance")
