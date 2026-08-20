# S10-T3 · Add the smallest coherent persistent desktop shell

**Layer:** Company Runtime image and mature process supervision.

**Observed friction:** Openbox can place windows but exposes no task switching or application entry.
After Chromium opens, another GUI application is effectively hidden from an ordinary owner.

## Outcome

The persistent Runtime starts Openbox, a compact Tint2 taskbar, maximised Chromium and the existing
browser broker under Supervisor. PCManFM provides ordinary access to durable company locations.
Restarting the shell preserves the company home and browser profile.

## Acceptance

- Chromium starts maximised after the taskbar establishes its work area.
- Tint2 shows running applications and launchers for the supervised browser and PCManFM.
- The company home exposes recognisable Downloads, Projects and Outputs links without moving or
  copying source-owned files.
- PCManFM opens, task switching works, and Chromium returns to the same profile/tab state.
- Supervisor reports every imported desktop process honestly; a failed panel degrades its service
  inventory rather than being hidden.
- No GNOME, KDE, Xfce, LXQt, custom desktop environment or new Runtime lifecycle is introduced.

## Prior machinery made deletable

- the assumption that a single permanently foreground Chromium window is the whole desktop UX;
- any plan to import a full desktop environment before a smaller shell is tried.
