use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use tauri::AppHandle;
use uuid::Uuid;

use super::{
    process,
    tools::MediaTools,
    types::{AppError, DownloadJob, DownloadRequest},
};

pub const MAX_ACTIVE_DOWNLOADS: usize = 3;

#[derive(Default)]
pub struct DownloadManager {
    active: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl DownloadManager {
    pub fn start(
        &self,
        app: AppHandle,
        tools: MediaTools,
        request: DownloadRequest,
    ) -> Result<DownloadJob, AppError> {
        let id = Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        self.reserve(&id, Arc::clone(&cancel))?;

        let (job, worker) =
            match process::start_download(app, tools, request, id.clone(), Arc::clone(&cancel)) {
                Ok(result) => result,
                Err(error) => {
                    self.remove(&id);
                    return Err(error);
                }
            };

        let active_jobs = Arc::clone(&self.active);
        let cleanup_id = id.clone();
        let cleanup = std::thread::Builder::new()
            .name(format!("clipkeep-job-cleanup-{id}"))
            .spawn(move || {
                let _ = worker.join();
                if let Ok(mut active) = active_jobs.lock() {
                    active.remove(&cleanup_id);
                }
            });
        if cleanup.is_err() {
            cancel.store(true, Ordering::SeqCst);
            self.remove(&id);
            return Err(AppError::new(
                "job_manager_failed",
                "Could not track the download job.",
                true,
            ));
        }

        Ok(job)
    }

    pub fn cancel(&self, id: &str) -> Result<(), AppError> {
        let active = self.active.lock().map_err(|_| {
            AppError::new(
                "job_manager_unavailable",
                "The download manager is unavailable.",
                true,
            )
        })?;
        match active.get(id) {
            Some(cancel) => {
                cancel.store(true, Ordering::SeqCst);
                Ok(())
            }
            None => Err(AppError::new(
                "job_not_found",
                "That download is no longer active.",
                false,
            )),
        }
    }

    fn reserve(&self, id: &str, cancel: Arc<AtomicBool>) -> Result<(), AppError> {
        let mut active = self.active.lock().map_err(|_| {
            AppError::new(
                "job_manager_unavailable",
                "The download manager is unavailable.",
                true,
            )
        })?;
        if active.len() >= MAX_ACTIVE_DOWNLOADS {
            return Err(AppError::new(
                "download_limit_reached",
                format!(
                    "ClipKeep can run up to {MAX_ACTIVE_DOWNLOADS} downloads at once. Wait for one to finish or cancel it."
                ),
                true,
            ));
        }
        active.insert(id.to_string(), cancel);
        Ok(())
    }

    fn remove(&self, id: &str) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DownloadManager, MAX_ACTIVE_DOWNLOADS};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    #[test]
    fn manager_accepts_multiple_jobs_up_to_the_limit() {
        let manager = DownloadManager::default();
        for index in 0..MAX_ACTIVE_DOWNLOADS {
            manager
                .reserve(&format!("job-{index}"), Arc::new(AtomicBool::new(false)))
                .unwrap();
        }

        let error = manager
            .reserve("over-limit", Arc::new(AtomicBool::new(false)))
            .unwrap_err();
        assert_eq!(error.code, "download_limit_reached");
    }

    #[test]
    fn cancelling_one_job_does_not_cancel_its_siblings() {
        let manager = DownloadManager::default();
        let first = Arc::new(AtomicBool::new(false));
        let second = Arc::new(AtomicBool::new(false));
        manager.reserve("first", Arc::clone(&first)).unwrap();
        manager.reserve("second", Arc::clone(&second)).unwrap();

        manager.cancel("first").unwrap();

        assert!(first.load(Ordering::SeqCst));
        assert!(!second.load(Ordering::SeqCst));
    }
}
