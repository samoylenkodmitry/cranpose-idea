# Preview gutter lifetime

During installed 0.7.1 validation, a Unicode live-value edit followed by Undo and
Save produced an EDT exception in RustRover's gutter painter: `getIconWidth()` was
called on a null icon. The source edit, preview and Undo worked, but the painter
retained an old renderer after its highlighter and Rust callback were disposed.

Studio 0.7.2 pins the shared Rust SDK fix. Icon, identity and action-thread
properties now work independently of callback lifetime. Native callbacks still
release project and source state when decorations are removed. Stale click actions
do nothing; no retained-callback workaround or unbounded cache was added.

The real IDE regression holds renderer/action references across disposal and
checks a usable icon, stable identity, action metadata, released callbacks and
inert action execution. Existing Unicode offset checks remain.

This is a correctness follow-up to the [0.7.1 parsing improvement](performance-0.7.1.md).
No additional performance gain is claimed. Application release builds are
unaffected; the fix is entirely in the Rust plugin host.
