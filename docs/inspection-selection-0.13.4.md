# Selection while inspection is hidden or paused

In signed Studio 0.13.3, selecting Sun, closing Inspect, and opening Mercury
left Sun's rectangle on Mercury's page. If Pick was active, hiding Inspect also
left it intercepting application clicks and requesting snapshots.

Studio 0.13.4 treats bounds and picking as live inspection features:

- Closing Inspect removes bounds and disables Pick. The selected identity and
  captured details remain available when inspection is reopened.
- Pausing preserves the captured inspector details but removes the rectangle
  from the running application. Pick is inactive while paused.
- Reopening or resuming waits for a fresh snapshot before restoring bounds.
  Replies issued before the transition cannot make old bounds current again.
- Activating Pick opens and resumes inspection, then waits for a fresh snapshot.
- A fresh snapshot that no longer contains the selected node clears selection.

The shared template's `SequenceGate::discard_pending` reserves a response
watermark above all issued requests. Clones share that watermark, including when
an unchanged model update is discarded. Request IDs continue increasing.
The template's native `inspection-profile --exercise-selection` regression
exercises the real controller and layout protocol.

## Validation and limits

The new native regression fails against the signed 0.13.3 renderer because the
hidden layout still contains both selected bounds and active Pick. The candidate
passes hide/reopen, pause/resume, moved-bounds and removed-node checks.
It issues no snapshot requests during a 1.1-second hidden observation. Existing
1,000-node filtering, scrolling, selection and responsive resize checks pass too.
Rust model tests cover late replies, unchanged fresh snapshots and disconnection.

This fixes selection lifecycle. It does not establish an edit-to-display,
compilation, startup or CPU speedup. No timer or application release change is
added. The application continues running while inspector details are paused.
