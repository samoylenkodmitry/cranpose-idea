# Shared UI and responsive inspector state

Studio now consumes the template's `cranpose-plugin-ui` crate for the IDE palette,
editor helpers, action buttons and background-task output. The Build panel retains
only its target/device/package logic. Its host delegates job messages through the
shared non-modal queue, preserving document-save and cancellation ordering.

The template's runnable **Scan project** example uses the same components without
depending on Studio or Cranpose Build. See the
[SDK guide](https://github.com/samoylenkodmitry/cranpose-intellij-plugin-template/blob/main/docs/shared-ui-and-jobs.md).

## Inspector lifetime

Local IDEA validation exposed an intermittent renderer panic when moving the
inspector between the side and bottom panels. A symbol-bearing renderer traced
it to `ScrollNode::on_detach` accessing a disposed `ScrollStateInner` during
subcomposition cleanup. Inspector controls, tree and details scroll state now
belong to the persistent controller scope, above the responsive branches. The
last filter value lives there too, so a placement change does not reset scrolling.

The shared Rust `inspection-profile --resize-rounds` check resizes the disconnected
controller, then scrolls a populated tree and resizes again. Every transition
requires matching viewport geometry and a fresh rendered inspection. The previous
renderer fails its scroll-retention assertion. The candidate passed 40 transitions
locally and two complete actual IDEA suites, including controller restart and
responsive placement. CI runs this check on macOS and Ubuntu.

These are correctness checks, not performance measurements. They do not establish
a before/after crash frequency or change the framework's general disposal order.
Application release builds and hot-reload instrumentation are unchanged.
