use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

const MAX_JOB_ID_LEN: usize = 128;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExclusiveJobError {
    #[error("job.invalid_id:Job ID must be between 1 and 128 characters.")]
    InvalidId,
    #[error("{code}:{summary}")]
    Busy {
        code: &'static str,
        summary: &'static str,
    },
    #[error("job.id_mismatch:The cancellation request does not match the active job.")]
    IdMismatch,
    #[error("job.registry_unavailable:The job registry is unavailable.")]
    RegistryUnavailable,
}

struct ActiveJob {
    id: String,
    cancel: Arc<AtomicBool>,
}

#[derive(Default)]
struct RegistryState {
    active: Option<ActiveJob>,
    pending_cancel: Option<String>,
}

struct RegistryInner {
    state: Mutex<RegistryState>,
    busy_code: &'static str,
    busy_summary: &'static str,
}

#[derive(Clone)]
pub struct ExclusiveJobRegistry {
    inner: Arc<RegistryInner>,
}

impl ExclusiveJobRegistry {
    pub fn new(busy_code: &'static str, busy_summary: &'static str) -> Self {
        Self {
            inner: Arc::new(RegistryInner {
                state: Mutex::new(RegistryState::default()),
                busy_code,
                busy_summary,
            }),
        }
    }

    pub fn acquire(&self, job_id: &str) -> Result<ExclusiveJobLease, ExclusiveJobError> {
        validate_job_id(job_id)?;
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ExclusiveJobError::RegistryUnavailable)?;
        if state.active.is_some() {
            return Err(ExclusiveJobError::Busy {
                code: self.inner.busy_code,
                summary: self.inner.busy_summary,
            });
        }

        let cancelled = state.pending_cancel.take().as_deref() == Some(job_id);
        let cancel = Arc::new(AtomicBool::new(cancelled));
        state.active = Some(ActiveJob {
            id: job_id.to_string(),
            cancel: Arc::clone(&cancel),
        });
        Ok(ExclusiveJobLease {
            inner: Arc::clone(&self.inner),
            job_id: job_id.to_string(),
            cancel,
        })
    }

    pub fn cancel(&self, job_id: &str) -> Result<bool, ExclusiveJobError> {
        validate_job_id(job_id)?;
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| ExclusiveJobError::RegistryUnavailable)?;
        match state.active.as_ref() {
            Some(active) if active.id == job_id => {
                active.cancel.store(true, Ordering::SeqCst);
                Ok(true)
            }
            Some(_) => Err(ExclusiveJobError::IdMismatch),
            None => {
                state.pending_cancel = Some(job_id.to_string());
                Ok(false)
            }
        }
    }
}

pub struct ExclusiveJobLease {
    inner: Arc<RegistryInner>,
    job_id: String,
    cancel: Arc<AtomicBool>,
}

impl ExclusiveJobLease {
    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn cancel_flag(&self) -> &AtomicBool {
        &self.cancel
    }
}

impl Drop for ExclusiveJobLease {
    fn drop(&mut self) {
        let Ok(mut state) = self.inner.state.lock() else {
            tracing::error!(job_id = %self.job_id, "exclusive job registry lock poisoned during release");
            return;
        };
        if state
            .active
            .as_ref()
            .is_some_and(|active| active.id == self.job_id)
        {
            state.active = None;
        }
    }
}

fn validate_job_id(job_id: &str) -> Result<(), ExclusiveJobError> {
    if job_id.trim().is_empty()
        || job_id.len() > MAX_JOB_ID_LEN
        || job_id.chars().any(char::is_control)
    {
        Err(ExclusiveJobError::InvalidId)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> ExclusiveJobRegistry {
        ExclusiveJobRegistry::new("job.test_busy", "A test job is already running.")
    }

    #[test]
    fn lease_is_exclusive_and_releases_with_raii() {
        let registry = registry();
        let first = registry.acquire("first").unwrap();
        let busy = match registry.acquire("second") {
            Ok(_) => panic!("a second lease must be rejected"),
            Err(error) => error,
        };
        assert_eq!(
            busy.to_string(),
            "job.test_busy:A test job is already running."
        );
        assert!(!first.cancel_flag().load(Ordering::SeqCst));
        drop(first);
        assert_eq!(registry.acquire("second").unwrap().job_id(), "second");
    }

    #[test]
    fn cancellation_is_exact_and_supports_cancel_before_acquire() {
        let registry = registry();
        assert!(!registry.cancel("pending").unwrap());
        assert!(!registry.cancel("pending").unwrap());
        let pending = registry.acquire("pending").unwrap();
        assert!(pending.cancel_flag().load(Ordering::SeqCst));
        drop(pending);

        assert!(!registry.cancel("stale").unwrap());
        let active = registry.acquire("successor").unwrap();
        assert!(!active.cancel_flag().load(Ordering::SeqCst));
        assert_eq!(registry.cancel("stale"), Err(ExclusiveJobError::IdMismatch));
        assert!(!active.cancel_flag().load(Ordering::SeqCst));
        assert!(registry.cancel("successor").unwrap());
        assert!(active.cancel_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn stale_lease_cannot_release_a_successor() {
        let registry = registry();
        let stale = registry.acquire("stale").unwrap();
        let successor_cancel = Arc::new(AtomicBool::new(false));
        registry.inner.state.lock().unwrap().active = Some(ActiveJob {
            id: "successor".to_string(),
            cancel: Arc::clone(&successor_cancel),
        });

        drop(stale);

        let state = registry.inner.state.lock().unwrap();
        assert_eq!(
            state.active.as_ref().map(|active| active.id.as_str()),
            Some("successor")
        );
        assert!(!successor_cancel.load(Ordering::SeqCst));
    }

    #[test]
    fn invalid_ids_and_poisoned_registry_fail_closed() {
        let registry = registry();
        assert!(matches!(
            registry.acquire(""),
            Err(ExclusiveJobError::InvalidId)
        ));
        assert!(matches!(
            registry.acquire("   "),
            Err(ExclusiveJobError::InvalidId)
        ));
        assert_eq!(
            registry.cancel(&"x".repeat(129)),
            Err(ExclusiveJobError::InvalidId)
        );

        let poisoned = registry.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.inner.state.lock().unwrap();
            panic!("poison registry");
        })
        .join();
        assert!(matches!(
            registry.acquire("later"),
            Err(ExclusiveJobError::RegistryUnavailable)
        ));
        assert_eq!(
            registry.cancel("later"),
            Err(ExclusiveJobError::RegistryUnavailable)
        );
    }

    #[test]
    fn independent_registries_do_not_contend() {
        let first = registry();
        let second = registry();
        let _first = first.acquire("a").unwrap();
        let _second = second.acquire("b").unwrap();
    }
}
