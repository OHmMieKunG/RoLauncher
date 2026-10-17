-- Install through Studio in StarterPlayerScripts; ordinary game code, no client injection.
local remote = game:GetService("ReplicatedStorage"):WaitForChild("RoLauncherAcceptanceHeartbeat", 30)
if not remote then return end
local input = game:GetService("UserInputService")
local player = game:GetService("Players").LocalPlayer
local inputs, idles = 0, 0
-- Counts only: never collect key codes, text or mouse coordinates.
input.InputBegan:Connect(function() inputs = math.min(inputs + 1, 1000000) end)
player.Idled:Connect(function() idles = math.min(idles + 1, 1000000) end)
while true do
    task.wait(10)
    remote:FireServer(inputs, idles)
end
