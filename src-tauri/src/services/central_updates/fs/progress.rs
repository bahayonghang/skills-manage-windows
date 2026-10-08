use std::time::{Duration, Instant};

use tokio::sync::watch;

const WRITE_PROGRESS_FILES: usize = 256;
const WRITE_PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StageWriteProgress {
    pub(crate) completed_files: usize,
    pub(crate) completed_bytes: u64,
    pub(crate) current_path: Option<String>,
}

/// Retains one cumulative value. The blocking worker owns no event handle.
#[derive(Debug, Clone)]
pub(crate) struct StageWriteProgressReporter {
    sender: watch::Sender<StageWriteProgress>,
    #[cfg(test)]
    observations: Option<std::sync::Arc<std::sync::Mutex<Vec<StageWriteProgress>>>>,
}

impl StageWriteProgressReporter {
    pub(crate) fn channel() -> (Self, watch::Receiver<StageWriteProgress>) {
        let (sender, receiver) = watch::channel(StageWriteProgress::default());
        (
            Self {
                sender,
                #[cfg(test)]
                observations: None,
            },
            receiver,
        )
    }

    pub(super) fn writer(&self) -> StageWriteProgressWriter<'_> {
        StageWriteProgressWriter {
            reporter: self,
            completed_files: 0,
            completed_bytes: 0,
            published_files: 0,
            published_at: Instant::now(),
            current_path: None,
        }
    }

    fn publish(&self, progress: StageWriteProgress) {
        #[cfg(test)]
        if let Some(observations) = &self.observations {
            observations.lock().unwrap().push(progress.clone());
        }
        self.sender.send_replace(progress);
    }
}

pub(super) struct StageWriteProgressWriter<'a> {
    reporter: &'a StageWriteProgressReporter,
    completed_files: usize,
    completed_bytes: u64,
    published_files: usize,
    published_at: Instant,
    current_path: Option<String>,
}

impl StageWriteProgressWriter<'_> {
    pub(super) fn written(&mut self, path: &str, bytes: u64) {
        self.completed_files += 1;
        self.completed_bytes += bytes;
        let current_path = self.current_path.get_or_insert_with(String::new);
        current_path.clear();
        current_path.push_str(path);
        if self.completed_files - self.published_files >= WRITE_PROGRESS_FILES
            || self.published_at.elapsed() >= WRITE_PROGRESS_INTERVAL
        {
            self.flush();
        }
    }

    pub(super) fn flush(&mut self) {
        self.reporter.publish(StageWriteProgress {
            completed_files: self.completed_files,
            completed_bytes: self.completed_bytes,
            current_path: self.current_path.clone(),
        });
        self.published_files = self.completed_files;
        self.published_at = Instant::now();
    }
}

#[cfg(test)]
pub(super) fn observe_parent_creation(path: &std::path::Path, succeeded: bool) {
    tests::observe_parent_creation(path, succeeded);
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    use super::*;
    use crate::services::central_updates::fs::{write_remote_skill_files, RemoteSkillFile};

    type ParentCreations = Rc<RefCell<Vec<(PathBuf, bool)>>>;
    thread_local! {
        static PARENT_CREATIONS: RefCell<Option<ParentCreations>> = const { RefCell::new(None) };
    }

    struct ParentCreationGuard(Option<ParentCreations>);

    impl ParentCreationGuard {
        fn start(observations: ParentCreations) -> Self {
            Self(PARENT_CREATIONS.with(|slot| slot.replace(Some(observations))))
        }
    }

    impl Drop for ParentCreationGuard {
        fn drop(&mut self) {
            PARENT_CREATIONS.with(|slot| slot.replace(self.0.take()));
        }
    }

    pub(super) fn observe_parent_creation(path: &Path, succeeded: bool) {
        PARENT_CREATIONS.with(|slot| {
            if let Some(observations) = slot.borrow().as_ref() {
                observations
                    .borrow_mut()
                    .push((path.to_path_buf(), succeeded));
            }
        });
    }

    fn file(path: &str, bytes: &[u8]) -> RemoteSkillFile {
        RemoteSkillFile {
            repo_path: format!("skills/test/{path}"),
            relative_path: path.to_string(),
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn progress_counts_only_successful_writes_and_flushes_the_first_error() {
        let root = tempfile::tempdir().unwrap();
        let parents = ParentCreations::default();
        let _guard = ParentCreationGuard::start(parents.clone());
        let (reporter, receiver) = StageWriteProgressReporter::channel();
        assert_eq!(receiver.borrow().completed_files, 0);
        let files = vec![
            file("a", b"first"),
            file("a/child", b"fails"),
            file("later", b"unused"),
        ];
        assert!(write_remote_skill_files(&files, root.path(), Some(&reporter)).is_err());
        assert_eq!(receiver.borrow().completed_files, 1);
        assert_eq!(receiver.borrow().completed_bytes, 5);
        assert_eq!(receiver.borrow().current_path.as_deref(), Some("a"));
        assert_eq!(std::fs::read(root.path().join("a")).unwrap(), b"first");
        assert!(!root.path().join("later").exists());
        assert_eq!(
            *parents.borrow(),
            [
                (root.path().to_path_buf(), true),
                (root.path().join("a"), false)
            ]
        );
    }

    #[test]
    fn unsafe_path_has_zero_completed_writes() {
        let root = tempfile::tempdir().unwrap();
        let parents = ParentCreations::default();
        let _guard = ParentCreationGuard::start(parents.clone());
        let (reporter, receiver) = StageWriteProgressReporter::channel();
        let error =
            write_remote_skill_files(&[file("../escape", b"bad")], root.path(), Some(&reporter))
                .unwrap_err();
        assert!(matches!(
            error,
            crate::services::central_updates::CentralUpdatesError::UnsupportedRepoFilePath(_)
        ));
        assert_eq!(*receiver.borrow(), StageWriteProgress::default());
        assert_eq!(*parents.borrow(), [(root.path().to_path_buf(), true)]);
    }

    #[test]
    fn many_files_publish_bounded_monotone_aggregates_and_an_exact_tail() {
        let observations = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (mut reporter, receiver) = StageWriteProgressReporter::channel();
        reporter.observations = Some(observations.clone());
        let mut writer = reporter.writer();
        let started = Instant::now();
        for index in 0..13_016 {
            writer.written(&format!("assets/{index}.bin"), 1_024);
        }
        writer.flush();
        let observations = observations.lock().unwrap();
        assert!(
            observations.len()
                <= 13_016_usize.div_ceil(256)
                    + started.elapsed().as_millis().div_ceil(100) as usize
                    + 2
        );
        assert!(observations
            .windows(2)
            .all(|pair| pair[0].completed_files <= pair[1].completed_files
                && pair[0].completed_bytes <= pair[1].completed_bytes));
        assert_eq!(receiver.borrow().completed_files, 13_016);
        assert_eq!(receiver.borrow().completed_bytes, 13_016 * 1_024);
    }

    #[test]
    fn time_threshold_publishes_before_a_full_file_batch() {
        let (reporter, receiver) = StageWriteProgressReporter::channel();
        let mut writer = reporter.writer();
        writer.published_at -= WRITE_PROGRESS_INTERVAL;
        writer.written("small.bin", 7);
        assert_eq!(receiver.borrow().completed_files, 1);
        assert_eq!(receiver.borrow().completed_bytes, 7);
    }

    #[test]
    fn shared_parent_preserves_all_files_and_content() {
        let root = tempfile::tempdir().unwrap();
        let parents = ParentCreations::default();
        let _guard = ParentCreationGuard::start(parents.clone());
        let files = (0..8)
            .map(|index| file(&format!("assets/shared/{index}"), &[index]))
            .collect::<Vec<_>>();
        write_remote_skill_files(&files, root.path(), None).unwrap();
        assert_eq!(
            *parents.borrow(),
            [
                (root.path().to_path_buf(), true),
                (root.path().join("assets/shared"), true)
            ]
        );
        for (index, file) in files.iter().enumerate() {
            assert_eq!(
                std::fs::read(root.path().join(&file.relative_path)).unwrap(),
                [index as u8]
            );
        }
    }
}
