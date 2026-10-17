# True no-render research

Status: **unverified and unavailable**, 2026-10-17. Dedicated accounts and an authorized experience are not ready; the user requested tooling and a blocked-case report.

Success means stopping ongoing rendering while the connected client continues simulation and observable game progress. Launching without graphics initialization is a stronger, separately unverified claim.

| Candidate | Classification | Evidence and limit |
| --- | --- | --- |
| Documented Roblox graphics controls | Unsupported for a rendering-off claim | [Roblox documents automatic/manual graphics quality](https://en.help.roblox.com/hc/en-us/articles/203314310-Graphics-Quality); it does not establish a player rendering-off interface. |
| Minimize / occlusion | Unverified | [ShowWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow) changes window presentation state. No live render, simulation, restoration, or connection observations were collected. |
| Lower priority / affinity | Unsupported as a rendering-off control | These schedule CPU work; they do not disable the renderer. Available only as resource controls. |
| Hide window | Unsupported as proof | Per-account Headless hides the window; this cannot establish rendering suppression. |
| Launch without graphics initialization | Unverified | No documented supported player capability was established within the allowed controls. |
| Injection, executors, memory writes, executable patches, client settings-file modifications | Excluded | Outside the permitted implementation. |

[PresentMon](https://github.com/GameTechDev/PresentMon) collects graphics presentation/frame telemetry. A lack of presentation events alone cannot establish that all rendering stopped; GPU/CPU activity also needs attribution and an independently observed active session.

## Reproducible acceptance gate

Use a separately selected disposable client, recording its PID, creation time, image path, tracker, and account generation without cookies or launch URLs. Never select an unmanaged client automatically.

1. Capture a visible positive control, two-second normalized CPU/working-set/busiest-GPU-engine samples, and independently attributable presentation and render-work telemetry.
2. Apply one candidate, retain raw measurement artifacts, and record connection and simulation/progress independently of process uptime.
3. Restore visibility/control state and repeat the positive control. Repeat the transition to show restoration is reliable.
4. Run separate 25-minute recovery-off and 60-minute recovery-on observations with independent progress and connection evidence; keep each connection interval separate.
5. Missing counters, access denial, zero GPU readings, or lost presentation telemetry leave the candidate unverified. Active render work fails the candidate.

No candidate passed this gate. Fixture behavior and OS key submission are separate evidence and do not prove Roblox rendering or input acceptance.

The test-only harness includes an opaque external occlusion cover with visible-before/after controls.
[DXGI occlusion](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/dxgi-status) lets an application choose to reduce rendering; flip-model presentation has different occlusion behavior.
Scheduling priority and Windows QoS are not rendering switches; no supported universal external renderer-off mechanism was established.

See [acceptance tooling](client-acceptance.md) and [actual host/evidence status](client-compatibility-report.md).

Headless now automatically requests EcoQoS, lower CPU/memory priority and minimize-before-hide.
The native Windows fixture verifies these controls and restoration; it is not a Roblox rendering or resource-savings result.
No renderer-off candidate was promoted by this change.

## Per-account 1 FPS and zero rendering research

No documented external PID-targeted Roblox frame limiter or rendering-off interface was established.
[DXGI frame latency](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_3/nf-dxgi1_3-idxgiswapchain2-setmaximumframelatency) limits queued frames, not frames per second, and requires the application's swap chain.
[DWM cloaking](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwmwindowattribute) keeps the window composed; freezing a thumbnail is not freezing the renderer.

[NVIDIA's documented background limiter](https://www.nvidia.com/content/Control-Panel-Help/vLatest/en-gb/mergedProjects/nv3dENG/Manage_3D_Settings_%28reference%29.htm) supports 20?200 FPS through application profiles.
It is excluded because executable-wide settings may affect Normal accounts sharing the image, including background Normal windows.
[Driver profile APIs](https://docs.nvidia.com/nvapi/group__drsapi.html) do not establish safe PID-targeted live switching for this launcher.

[EcoQoS and timer-resolution policy](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setprocessinformation) can target one process but do not disable its renderer.
The latter and EcoQoS-off remain explicit acceptance experiments; no live result is available.
Zero rendering and per-process 1 FPS remain unsupported under the current verified interfaces; Network-only remains unsupported.
