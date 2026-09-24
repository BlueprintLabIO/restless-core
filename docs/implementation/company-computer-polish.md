# Company computer polish

The existing company desktop works, but its stock remote-view controls, small
application chrome and duplicated instructions make it feel separate from Restless.
This change keeps the persistent Linux computer and gives it one consistent surface.

## Delivery scope

- A shared direct noVNC viewer for Computer and prepared Attention handoffs.
- Clear viewing/control state, reconnect feedback, fullscreen and text clipboard controls.
- One live viewer connection through viewing and control changes. The primary
  attached viewer sizes the shared framebuffer; the active human controller has
  priority over other viewers.
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

- Production frontend and daemon builds passed. The current Svelte check reports
  zero errors and warnings; the separate type convention check also passed.
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

## Live sizing and control, 24 September 2026

The first observer now claims a short display lease and sizes the shared TigerVNC
framebuffer to its available area. Another viewer may watch without changing the
display. A human who takes control gets sizing priority until release. noVNC
stays connected while the user switches between viewing and control. The first
click claims control and reaches the Linux desktop; a second tab cannot send
input while the first tab controls it.

The live disposable-company check measured a 1278 × 620 framebuffer inside a
1278 × 620 screen. Resizing the browser changed both, in place, to 1098 × 700
and back to 1278 × 620 without a page refresh or letterboxing. The WebSocket
parser now handles noVNC's signed extended-clipboard length and full
SetDesktopSize frame; both errors had previously held up later input and resize
messages. At higher display density, the client requests CSS size multiplied by
device-pixel ratio, bounded to 3840 × 2160. The current in-app browser smoke
ran at device-pixel ratio 1; a true high-density display still needs direct
visual confirmation.

There is still one shared framebuffer per company computer. When attached
viewers have different window sizes, the primary viewer or active controller
sets its size; the others see a scaled view until they become primary.
