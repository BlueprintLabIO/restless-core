# Company computer polish

The existing company desktop works, but its stock remote-view controls, small
application chrome and duplicated instructions make it feel separate from Restless.
This change keeps the persistent Linux computer and gives it one consistent surface.

## Delivery scope

- A shared direct noVNC viewer for Computer and prepared Attention handoffs.
- Clear viewing/control state, reconnect feedback, fullscreen and text clipboard controls.
- Controller-only framebuffer resizing; observers scale their own view.
- A selector for actual open applications, with foregrounding tied to the requesting
  tab's current owner lease. Installed applications also remain reachable from the desktop.
- Readable Linux fonts, window chrome, taskbar, cursor and terminal defaults.
- Existing company files, browser profiles and user appearance overrides survive upgrades.

Linux, Openbox, Chromium, TigerVNC and noVNC remain the stack. Browser-native outcome
previews remain separate from the shared desktop. Multiple machines, GPU acceleration,
audio transport and a new video-streaming protocol are outside this change.

## Visual direction

Use Bridge Light for the surrounding controls and restrained slate for the desktop.
Beautiful UI's compact state presentation, Cult UI's continuity between contained and
expanded surfaces, and Origin UI Svelte's ordinary application controls inform the polish.
No upstream component code or additional UI framework is imported.

## Verification

Build the frontend and daemon once the integrated slice is ready. Use a disposable
company for input, window switching and appearance checks, then verify read-only attachment
to the real company. Check desktop and narrow layouts, controller/observer separation,
reconnection, fullscreen and clipboard. Record actual outcomes below before release.

### Verified locally, 24 September 2026

- Production frontend build and combined daemon build passed. The frontend type
  checker still reports pre-existing duplicate TanStack QueryClient type errors in
  document code; it did not expose a remaining error in this desktop slice.
- Used the real `lantern_live_film_test` computer, not a mocked desktop. Reconciled
  its image through the normal lifecycle; its persistent volume and browser profile
  remained intact.
- Opened Godot and the terminal. The Restless application selector enumerated real
  windows and brought Chromium and Godot to the foreground. The terminal opened
  within a laptop-height viewport with readable text and no patterned scrollbar.
- Took and released control, used fullscreen, and transferred text with the explicit
  clipboard panel. Pasting through Chromium's own context menu confirmed the text
  reached the remote clipboard. Observer mode disables application switching.
- Restarted only the disposable computer's desktop-web process. The viewer
  automatically reconnected with the same applications still open.
- Checked the deployed static frontend at desktop size and 390px width. Header
  controls wrap into clear rows; clipboard, scaling and fullscreen remain reachable
  beside the floating Exec control.
- Unattached window API requests are rejected. Focus also verifies the attached
  client and live lease in Core and again in the broker before activation.

The shared viewer is used by prepared Attention handoffs, but a new live Attention
approval flow was not exercised during this visual smoke. Godot's project manager
was exercised; GPU-heavy projects, video editing performance and audio were not.

The local runtime image was built as a small derivative of the existing compatible
image; the canonical Dockerfile contains the same defaults for future full builds.
Old hashed frontend assets and the previous desktop image were retained for rollback.

## Display sizing after the first live review

The observer keeps the shared 1280 × 800 framebuffer and scales it locally. On a
1450 × 1286 browser viewport, the 1448 × 1186 display area showed that image
as 1448 × 905, leaving broad bands above and below. The enlargement also
softened text. This follows the single-controller sizing rule; it is not an
inherent limit of TigerVNC or noVNC.

Taking control in the same tall viewport produced a 1448 × 1186 framebuffer in
a 1448 × 1186 display area, without local enlargement or letterboxing. Live
resizing during control also followed a 1198 × 700 display area pixel for pixel.
The UI now identifies a scaled observation, describes the actual-pixels option,
and explains that taking control uses the window's full resolution. Exec sits in
the desktop utility bar rather than floating above it. Desktop and 390px mobile
layouts were checked after deployment.

The shared computer still has one framebuffer. Two observers with differently
shaped windows cannot both fill their screens without changing global geometry,
cropping, or distortion. A future primary-viewer resolution policy would need to
arbitrate those changes around other humans and agents; this release keeps the
single-controller boundary.
