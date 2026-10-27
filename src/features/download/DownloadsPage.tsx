import {
  AlertCircle,
  ArrowDownToLine,
  AudioLines,
  CheckCircle2,
  Clock3,
  FileVideo2,
  RotateCcw,
  X,
} from "lucide-react";
import type { DownloadJob, DownloadStatus } from "../../types";

interface DownloadsPageProps {
  jobs: DownloadJob[];
  onCancel: (id: string) => void;
  onRetry: (job: DownloadJob) => void;
}

const STATUS_LABELS: Record<DownloadStatus, string> = {
  queued: "Queued",
  validating: "Checking link",
  fetching_metadata: "Reading details",
  downloading: "Downloading",
  merging: "Merging",
  converting: "Converting",
  finalizing: "Finalizing",
  completed: "Completed",
  failed: "Failed",
  cancelling: "Cancelling",
  cancelled: "Cancelled",
};

function isActive(status: DownloadStatus) {
  return !["completed", "failed", "cancelled"].includes(status);
}

export function DownloadsPage({ jobs, onCancel, onRetry }: DownloadsPageProps) {
  const active = jobs.filter((job) => isActive(job.status));
  const finished = jobs.filter((job) => !isActive(job.status));

  return (
    <div className="content-page downloads-page">
      <div className="page-intro">
        <div>
          <p className="section-kicker">YOUR QUEUE</p>
          <h1>Downloads</h1>
          <p className="page-subtitle">Track the status of your current and recent jobs.</p>
        </div>
        <div className="jobs-count">
          <span>{jobs.length.toString().padStart(2, "0")}</span>
          <small>this session</small>
        </div>
      </div>

      {jobs.length === 0 ? (
        <div className="empty-state">
          <div className="empty-state-icon"><ArrowDownToLine size={22} /></div>
          <h2>Your download list is clear</h2>
          <p>When you start a download, its progress and outcome will appear here.</p>
          <p className="empty-state-note">Preview mode does not download or save media.</p>
        </div>
      ) : (
        <>
          <section className="job-section" aria-labelledby="active-jobs-heading">
            <div className="list-section-heading">
              <h2 id="active-jobs-heading">In progress <span>{active.length}</span></h2>
              {active.length > 0 && <span className="section-status"><i /> ACTIVE</span>}
            </div>
            {active.length === 0 ? (
              <div className="section-empty">No active downloads right now.</div>
            ) : (
              <div className="job-list">
                {active.map((job) => (
                  <JobCard key={job.id} job={job} onCancel={onCancel} onRetry={onRetry} />
                ))}
              </div>
            )}
          </section>

          <section className="job-section" aria-labelledby="recent-jobs-heading">
            <div className="list-section-heading">
              <h2 id="recent-jobs-heading">Recent activity <span>{finished.length}</span></h2>
            </div>
            {finished.length === 0 ? (
              <div className="section-empty">Finished jobs will appear here.</div>
            ) : (
              <div className="job-list">
                {finished.map((job) => (
                  <JobCard key={job.id} job={job} onCancel={onCancel} onRetry={onRetry} />
                ))}
              </div>
            )}
          </section>
        </>
      )}
    </div>
  );
}

function JobCard({
  job,
  onCancel,
  onRetry,
}: {
  job: DownloadJob;
  onCancel: (id: string) => void;
  onRetry: (job: DownloadJob) => void;
}) {
  const Icon = job.format === "mp4" ? FileVideo2 : AudioLines;
  const active = isActive(job.status);
  const failed = job.status === "failed";
  const cancelled = job.status === "cancelled";
  const progress = job.progressPercent ?? 0;

  return (
    <article className={`job-card${failed ? " is-failed" : ""}${cancelled ? " is-cancelled" : ""}`}>
      <div className={`job-format-icon ${job.format}`}>
        <Icon size={19} aria-hidden="true" />
      </div>
      <div className="job-main">
        <div className="job-title-row">
          <div>
            <h3>{job.format === "mp4" ? "MP4 video" : "MP3 audio"} · {job.quality === "best" ? "Best available" : job.quality}</h3>
            <p>{new Date(job.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</p>
          </div>
          <span className={`job-status ${failed ? "status-failed" : cancelled ? "status-cancelled" : active ? "status-active" : "status-complete"}`}>
            {failed ? <AlertCircle size={14} /> : cancelled ? <X size={14} /> : active ? <Clock3 size={14} /> : <CheckCircle2 size={14} />}
            {STATUS_LABELS[job.status]}
          </span>
        </div>
        <div
          className="job-progress-track"
          role="progressbar"
          aria-label={`${STATUS_LABELS[job.status]} job progress`}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={job.progressPercent ?? undefined}
        >
          <span
            className={`job-progress-fill${failed ? " failed" : ""}`}
            style={{ width: `${progress}%` }}
          />
        </div>
        <div className="job-details-row">
          <span>
            {active
              ? job.progressPercent === null
                ? "Preparing…"
                : `${progress}%${job.bytesPerSecond === null ? "" : ` · ${formatSpeed(job.bytesPerSecond)}`}${job.etaSeconds === null ? "" : ` · ${formatDuration(job.etaSeconds)} left`}`
              : failed
                ? "No completed file saved"
                : cancelled
                  ? "Download cancelled"
                  : job.outputPath ?? "No file was created"}
          </span>
          {active && (
            <button className="job-action" type="button" onClick={() => onCancel(job.id)}>
              Cancel <X size={13} />
            </button>
          )}
          {failed && (
            <button className="job-action" type="button" onClick={() => onRetry(job)}>
              Paste link again <RotateCcw size={13} />
            </button>
          )}
        </div>
        {job.error && <p className="job-error" role="status">{job.error.message}</p>}
      </div>
    </article>
  );
}

function formatSpeed(bytesPerSecond: number): string {
  if (bytesPerSecond >= 1024 * 1024) {
    return `${(bytesPerSecond / (1024 * 1024)).toFixed(1)} MiB/s`;
  }
  if (bytesPerSecond >= 1024) {
    return `${(bytesPerSecond / 1024).toFixed(0)} KiB/s`;
  }
  return `${Math.round(bytesPerSecond)} B/s`;
}

function formatDuration(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return minutes > 0
    ? `${minutes}:${String(remainingSeconds).padStart(2, "0")}`
    : `${remainingSeconds}s`;
}
