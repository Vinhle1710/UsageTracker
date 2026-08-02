use crate::material::NativeWindowState;
use std::sync::{mpsc, Mutex};
use tauri::Emitter;

pub struct NativeSurfaceState {
    pub cache: Mutex<NativeWindowState>,
    pub deferred_repair_queued: std::sync::atomic::AtomicBool,
}

impl Default for NativeSurfaceState {
    fn default() -> Self {
        Self {
            cache: Mutex::new(NativeWindowState::default()),
            deferred_repair_queued: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

pub fn dispatch<R, F>(app: &tauri::AppHandle, operation: F) -> Result<R, String>
where
    R: Send + 'static,
    F: FnOnce() -> Result<R, String> + Send + 'static,
{
    let (sender, receiver) = mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = sender.send(operation());
    })
    .map_err(|error| format!("native surface dispatch failed: {error}"))?;
    receiver
        .recv()
        .map_err(|_| "native surface dispatch cancelled".to_string())?
}

pub fn report_diagnostic(app: &tauri::AppHandle, operation: &'static str, _error: &str) {
    let _ = app.emit(
        "native-surface-diagnostic",
        serde_json::json!({
            "operation": operation,
            "status": "failed",
        }),
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceOperation {
    Visibility {
        visible: bool,
    },
    Geometry(NativeWindowState),
    Repair {
        label: &'static str,
        force_region: bool,
    },
}

pub trait SurfaceExecutor {
    type Error;

    fn execute(
        &mut self,
        operation: SurfaceOperation,
        state: &mut NativeWindowState,
    ) -> Result<(), Self::Error>;
}

pub struct NativeSurfaceController<E> {
    executor: E,
    state: NativeWindowState,
    pending_repair: Option<(&'static str, bool)>,
}

impl<E> NativeSurfaceController<E> {
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            state: NativeWindowState::default(),
            pending_repair: None,
        }
    }

    pub fn dispatch(&mut self, operation: SurfaceOperation) -> Result<(), E::Error>
    where
        E: SurfaceExecutor,
    {
        let mut next_state = self.state.clone();
        if let SurfaceOperation::Geometry(next) = &operation {
            next_state = next.clone();
        }
        self.executor.execute(operation, &mut next_state)?;
        self.state = next_state;
        Ok(())
    }

    pub fn state(&self) -> &NativeWindowState {
        &self.state
    }

    pub fn executor(&self) -> &E {
        &self.executor
    }

    pub fn queue_repair(&mut self, label: &'static str, force_region: bool) -> bool {
        if self.pending_repair.is_some() {
            return false;
        }
        self.pending_repair = Some((label, force_region));
        true
    }

    pub fn complete_repair(&mut self) {
        self.pending_repair = None;
    }

    pub fn pending_repair(&self) -> Option<(&'static str, bool)> {
        self.pending_repair
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{CardRegion, Material, NativeMaterialSpec, NativeWindowState};

    #[derive(Default)]
    struct MockExecutor {
        operations: Vec<SurfaceOperation>,
        fail_next: bool,
    }

    impl SurfaceExecutor for MockExecutor {
        type Error = &'static str;

        fn execute(
            &mut self,
            operation: SurfaceOperation,
            _state: &mut NativeWindowState,
        ) -> Result<(), Self::Error> {
            self.operations.push(operation);
            if self.fail_next {
                self.fail_next = false;
                Err("native failure")
            } else {
                Ok(())
            }
        }
    }

    fn regions(width: i32) -> Vec<CardRegion> {
        vec![CardRegion {
            x: 0,
            y: 0,
            width,
            height: 20,
            radius: 8,
        }]
    }

    fn geometry(width: i32) -> NativeWindowState {
        NativeWindowState {
            material: Some(NativeMaterialSpec {
                material: Material::Clear,
                tint: (1, 2, 3, 4),
            }),
            regions: regions(width),
            size: Some((width as u32, 20)),
        }
    }

    #[test]
    fn geometry_updates_cached_state_before_a_later_repair_reads_it() {
        let mut controller = NativeSurfaceController::new(MockExecutor::default());

        controller
            .dispatch(SurfaceOperation::Geometry(geometry(200)))
            .unwrap();
        controller
            .dispatch(SurfaceOperation::Repair {
                label: "main",
                force_region: false,
            })
            .unwrap();

        assert_eq!(controller.state().regions, regions(200));
        assert_eq!(
            controller.executor().operations,
            vec![
                SurfaceOperation::Geometry(geometry(200)),
                SurfaceOperation::Repair {
                    label: "main",
                    force_region: false
                },
            ]
        );
    }

    #[test]
    fn visibility_geometry_and_repair_are_serialized_in_submission_order() {
        let mut controller = NativeSurfaceController::new(MockExecutor::default());

        controller
            .dispatch(SurfaceOperation::Visibility { visible: true })
            .unwrap();
        controller
            .dispatch(SurfaceOperation::Geometry(geometry(200)))
            .unwrap();
        controller
            .dispatch(SurfaceOperation::Repair {
                label: "main",
                force_region: false,
            })
            .unwrap();
        controller
            .dispatch(SurfaceOperation::Visibility { visible: false })
            .unwrap();

        assert_eq!(controller.executor().operations.len(), 4);
    }

    #[test]
    fn queued_repairs_deduplicate_without_capturing_stale_geometry() {
        let mut controller = NativeSurfaceController::new(MockExecutor::default());

        assert!(controller.queue_repair("main", false));
        assert!(!controller.queue_repair("main", true));
        controller
            .dispatch(SurfaceOperation::Geometry(geometry(300)))
            .unwrap();
        controller.complete_repair();
        assert!(controller.queue_repair("main", false));

        assert_eq!(controller.pending_repair(), Some(("main", false)));
    }

    #[test]
    fn executor_errors_are_returned_to_the_dispatcher_caller() {
        let executor = MockExecutor {
            fail_next: true,
            ..Default::default()
        };
        let mut controller = NativeSurfaceController::new(executor);

        assert_eq!(
            controller.dispatch(SurfaceOperation::Visibility { visible: true }),
            Err("native failure")
        );
    }
}
