// startup_stager.rs — Sprint 90 Load Performance
//
// Provides a simple frame-budget queue so expensive one-shot generation tasks
// (procedural textures, large scatter spawns) run a few items per frame
// instead of all at once.  This keeps each frame under ~16 ms and prevents the
// browser's main thread from going non-responsive during the load burst.
//
// Design
// ------
// A `StartupQueue` resource holds a `VecDeque<Box<dyn FnOnce(&mut World)>>`.
// Any plugin that wants to stagger heavy one-shot work registers a closure via
// `StartupQueue::push`.  A single `Update` system (`drain_startup_queue`) runs
// those closures one-at-a-time, one per frame, until the queue is empty.
//
// By doing work in `Update` (not `Startup`/`PostStartup`) each closure gets its
// own full Bevy frame — the browser's `requestAnimationFrame` callback returns
// between calls, keeping the tab responsive.
//
// Texture plug-in pattern
// -----------------------
// Texture generators that previously ran in `Startup` are split into two parts:
//   1. A lightweight `Startup` system that pushes a closure into `StartupQueue`.
//   2. The closure itself calls `World::resource_mut::<Assets<Image>>()` and
//      inserts the result, then `commands.insert_resource(...)` for the Handle.
//
// Materials that depend on the texture handle tolerate a one-frame delay
// because:
//   a) The texture handle is inserted into the resource *before* the material
//      is used (materials are applied lazily via `apply_vehicle_textures` in
//      Update, which already waits until vehicle entities exist).
//   b) For terrain / water the handle is stored in a Resource and only
//      referenced the next PostStartup — by which time at least one stagger
//      frame has run.
//
// For terrain specifically the Detail-Normal texture is consumed in
// `PostStartup` (spawn_terrain).  To guarantee it arrives in time, that
// texture is kept on the *first* slot pushed, so it runs frame 1.
//
// Public API
//   StartupStagerPlugin  — registers the resource + drain system
//   StartupQueue         — resource; callers push closures into it

use std::collections::VecDeque;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct StartupStagerPlugin;

impl Plugin for StartupStagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StartupQueue::default())
            .add_systems(Update, drain_startup_queue);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

type WorkFn = Box<dyn FnOnce(&mut World) + Send + Sync + 'static>;

/// A queue of deferred one-shot world-mutation closures.
///
/// Each call to `drain_startup_queue` pops and runs one closure so the
/// browser returns to its event loop between each expensive generation step.
#[derive(Resource, Default)]
pub struct StartupQueue {
    queue: VecDeque<WorkFn>,
}

impl StartupQueue {
    /// Enqueue a closure for deferred execution (one per frame).
    pub fn push<F>(&mut self, f: F)
    where
        F: FnOnce(&mut World) + Send + Sync + 'static,
    {
        self.queue.push_back(Box::new(f));
    }

    /// True once all queued work has been consumed.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Number of items still pending.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}

// ---------------------------------------------------------------------------
// Drain system — runs one closure per frame until the queue is empty.
// ---------------------------------------------------------------------------

fn drain_startup_queue(world: &mut World) {
    // Pop one item; if empty, do nothing.
    let work = {
        let mut queue = world.resource_mut::<StartupQueue>();
        queue.queue.pop_front()
    };

    let Some(f) = work else { return };
    f(world);

    // Log progress (only while queue is non-empty so it doesn't spam forever).
    let remaining = world.resource::<StartupQueue>().pending();
    if remaining > 0 {
        debug!("startup_stager: ran 1 deferred task, {} remaining", remaining);
    } else {
        info!("startup_stager: all deferred startup tasks complete");
    }
}
