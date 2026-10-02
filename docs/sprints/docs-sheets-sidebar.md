# Docs and Sheets navigation

Observed friction: artifact titles looked like ordinary underlined links, and
Docs and Sheets had different, weak navigation controls. This change gives both
surfaces the same compact company artifact browser, using the cockpit's existing
tokens and icons.

The sidebar owns creation entry, search controls, recent/name ordering, native
artifact links and useful loading, empty, no-results and retry states. Arrow keys
enter and move through results; Home/End move within the list; Enter opens a row;
Escape clears search or returns to navigation. The local `/` shortcut works only
inside the sidebar, preserving editor typing and global shortcuts. Mobile
navigation uses a full-width list and an explicit editor back control.

Docs uses the existing actor-scoped native document search, including older
documents and body matches in the current named version. Search pages are loaded
explicitly and deduplicated. Sheets filters its existing authorized list by title.
Sorting orders the currently loaded list or results. There are no new backend
routes, folders or persistent preferences.

Permission boundaries remain fail closed: stale query/company/principal
completions are discarded, authoritative company denial clears search results,
and revoked documents cannot return from a pending request. Transient search,
ordering and focus reset with the company/principal partition. A fresh search
rechecks permissions so a re-granted document can return. Spreadsheet mount
preserves a deliberate outside control's focus while respecting direct editor
entry and interaction during loading.

Verification uses the isolated `artifactnav` / `artifact_sidebar_test` stack,
with unchanged Rust services based on `0f27a7c` and the candidate frontend.
The focused search lifecycle harness, frontend type/UI checks and production
build pass. Native desktop/mobile review includes older and body-only search
hits, pagination, keyboard navigation, global shortcuts and editor focus.
No founder company or primary checkout is used.
