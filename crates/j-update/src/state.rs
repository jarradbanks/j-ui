//! Small explicit state machine: a late progress/result cannot revive a cancelled download.
use crate::network::{Available, Download};

#[derive(Debug, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    Current,
    Available(Available),
    Downloading {
        update: Available,
        received: u64,
    },
    Ready(Download),
    Failed(String),
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Work {
    pub unsaved: bool,
    pub recording: bool,
    /// Includes queued exports and cancelled jobs still winding down.
    pub exporting: bool,
    pub other_job: bool,
}

impl Work {
    pub fn can_install(self) -> bool {
        !self.unsaved && !self.recording && !self.exporting && !self.other_job
    }
}

impl State {
    pub fn busy(&self) -> bool {
        matches!(self, Self::Checking | Self::Downloading { .. })
    }

    pub fn progress(&mut self, received: u64) {
        if let Self::Downloading {
            received: current,
            update,
        } = self
        {
            *current = received.min(update.asset.size).max(*current);
        }
    }

    pub fn cancel(&mut self) {
        if let Self::Downloading { update, .. } = self {
            *self = Self::Available(update.clone());
        }
    }
}

/// Optional UI-independent worker owner. Iced can instead schedule `Client` directly.
/// Dropping a receiver cancels ownership: a late worker cannot restore cancelled state.
pub struct Updater {
    pub state: State,
    client: crate::Client,
    current: String,
    receiver: Option<std::sync::mpsc::Receiver<Event>>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
enum Event {
    Checked(crate::Result<Option<Available>>),
    Progress(u64),
    Downloaded(crate::Result<Download>),
}
impl Updater {
    pub fn new(client: crate::Client, current: impl Into<String>) -> Self {
        Self {
            state: State::Idle,
            client,
            current: current.into(),
            receiver: None,
            cancel: Default::default(),
        }
    }
    /// Checks never discard an already verified download.
    pub fn check(&mut self) {
        if self.state.busy() || matches!(self.state, State::Ready(_)) {
            return;
        }
        let client = self.client.clone();
        let current = self.current.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.receiver = Some(rx);
        self.state = State::Checking;
        std::thread::spawn(move || {
            let _ = tx.send(Event::Checked(client.check(&current)));
        });
    }
    pub fn download(&mut self) {
        let State::Available(update) = &self.state else {
            return;
        };
        let update = update.clone();
        self.cancel = Default::default();
        let cancel = self.cancel.clone();
        let client = self.client.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.receiver = Some(rx);
        self.state = State::Downloading {
            update: update.clone(),
            received: 0,
        };
        std::thread::spawn(move || {
            let result = client.download(update, &cancel, |n| {
                let _ = tx.send(Event::Progress(n));
            });
            let _ = tx.send(Event::Downloaded(result));
        });
    }
    pub fn cancel(&mut self) {
        if matches!(self.state, State::Downloading { .. }) {
            self.cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.receiver = None;
            self.state.cancel();
        }
    }
    /// Drain on the UI thread; returns whether any state changed. Poll while state is busy.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let mut done = false;
        if let Some(rx) = &self.receiver {
            loop {
                match rx.try_recv() {
                    Ok(Event::Progress(n)) => {
                        self.state.progress(n);
                        changed = true;
                    }
                    Ok(event) => {
                        self.state = match event {
                            Event::Checked(Ok(Some(u))) => State::Available(u),
                            Event::Checked(Ok(None)) => State::Current,
                            Event::Downloaded(Ok(d)) => State::Ready(d),
                            Event::Checked(Err(e)) | Event::Downloaded(Err(e)) => State::Failed(e),
                            Event::Progress(_) => unreachable!(),
                        };
                        changed = true;
                        done = true;
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.state =
                            State::Failed("Update worker stopped unexpectedly; try again.".into());
                        changed = true;
                        done = true;
                        break;
                    }
                }
            }
        }
        if done {
            self.receiver = None;
        }
        changed
    }
    /// Call after the app finishes its final save/checkpoint and re-evaluates active work.
    /// A deferral preserves Ready and its temporary files.
    pub fn prepare(
        &mut self,
        work: Work,
        paths: &crate::InstallPaths,
    ) -> Option<crate::Result<crate::install::Prepared>> {
        if !work.can_install() || !matches!(self.state, State::Ready(_)) {
            return None;
        }
        let State::Ready(download) = std::mem::take(&mut self.state) else {
            unreachable!()
        };
        let result = crate::install::prepare(self.client.config(), &self.current, download, paths);
        if let Err(e) = &result {
            self.state = State::Failed(e.clone());
        }
        Some(result)
    }
}
impl Drop for Updater {
    fn drop(&mut self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_kind_of_active_work_defers_installation() {
        assert!(Work::default().can_install());
        for work in [
            Work {
                unsaved: true,
                ..Work::default()
            },
            Work {
                recording: true,
                ..Work::default()
            },
            Work {
                exporting: true,
                ..Work::default()
            },
            Work {
                other_job: true,
                ..Work::default()
            },
        ] {
            assert!(!work.can_install(), "{work:?}");
        }
    }
}

#[cfg(test)]
mod worker_tests {
    use super::*;
    use std::sync::mpsc;
    fn update() -> Available {
        Available {
            version: "1.2.0".into(),
            notes: "notes".into(),
            asset: crate::metadata::Asset {
                target: crate::fixtures::TARGET.into(),
                distribution: "windows-installer".into(),
                name: "fixture.exe".into(),
                size: 100,
                sha256: "00".repeat(32),
            },
            asset_id: 1,
            tag: "v1.2.0".into(),
            manifest: vec![],
            signature: vec![],
        }
    }
    fn updater() -> Updater {
        Updater::new(
            crate::Client::new(crate::fixtures::config("jvid", &crate::fixtures::key(43))).unwrap(),
            "1.0.0",
        )
    }
    #[test]
    fn progress_ready_error_retry_and_cancelled_worker_results_have_explicit_states() {
        let mut u = updater();
        let (tx, rx) = mpsc::channel();
        u.receiver = Some(rx);
        u.state = State::Checking;
        tx.send(Event::Checked(Ok(Some(update())))).unwrap();
        assert!(u.poll());
        assert!(matches!(u.state, State::Available(_)));
        let (tx, rx) = mpsc::channel();
        u.receiver = Some(rx);
        u.state = State::Downloading {
            update: update(),
            received: 0,
        };
        for n in [30, 20, 1000] {
            tx.send(Event::Progress(n)).unwrap();
        }
        u.poll();
        assert!(matches!(u.state, State::Downloading { received: 100, .. }));
        u.cancel();
        assert!(u.cancel.load(std::sync::atomic::Ordering::Relaxed));
        let d = Download {
            update: update(),
            dir: tempfile::tempdir().unwrap(),
        };
        let stage = d.dir.path().to_owned();
        let late = tx.send(Event::Downloaded(Ok(d)));
        assert!(late.is_err());
        drop(late);
        assert!(!stage.exists());
        u.poll();
        assert!(matches!(u.state, State::Available(_)));
        let (tx, rx) = mpsc::channel();
        u.receiver = Some(rx);
        tx.send(Event::Downloaded(Err("disk full".into()))).unwrap();
        u.poll();
        assert!(matches!(u.state, State::Failed(_)) && u.receiver.is_none());
    }
    #[test]
    fn a_disconnected_worker_becomes_a_retryable_error() {
        let mut u = updater();
        let (tx, rx) = mpsc::channel();
        u.receiver = Some(rx);
        u.state = State::Checking;
        drop(tx);
        u.poll();
        assert!(matches!(u.state, State::Failed(_)) && u.receiver.is_none());
    }
    #[test]
    fn ready_download_survives_checks_and_work_deferral_and_is_cleaned_on_drop() {
        let mut u = updater();
        let d = Download {
            update: update(),
            dir: tempfile::tempdir().unwrap(),
        };
        let stage = d.dir.path().to_owned();
        u.state = State::Ready(d);
        u.check();
        assert!(matches!(u.state, State::Ready(_)) && u.receiver.is_none());
        let paths = crate::InstallPaths {
            executable: "absent.exe".into(),
            helper: "absent-helper.exe".into(),
            report: "report.txt".into(),
        };
        assert!(
            u.prepare(
                Work {
                    unsaved: true,
                    ..Default::default()
                },
                &paths
            )
            .is_none()
        );
        assert!(stage.exists());
        drop(u);
        assert!(!stage.exists());
    }

    #[test]
    fn preparation_failure_preserves_the_diagnostic_without_installing() {
        let mut u = updater();
        u.state = State::Ready(Download {
            update: update(),
            dir: tempfile::tempdir().unwrap(),
        });
        let paths = crate::InstallPaths {
            executable: "absent.exe".into(),
            helper: "absent-helper.exe".into(),
            report: "report.txt".into(),
        };
        assert!(u.prepare(Work::default(), &paths).unwrap().is_err());
        assert!(matches!(u.state, State::Failed(_)));
        assert!(u.prepare(Work::default(), &paths).is_none());
        assert!(matches!(u.state, State::Failed(_)));
    }
    #[test]
    fn jplay_every_kind_of_active_work_defers_installation() {
        assert!(Work::default().can_install());
        for work in [
            Work {
                unsaved: true,
                ..Default::default()
            },
            Work {
                recording: true,
                ..Default::default()
            },
            Work {
                exporting: true,
                ..Default::default()
            },
            Work {
                other_job: true,
                ..Default::default()
            },
        ] {
            assert!(!work.can_install());
        }
    }
}
