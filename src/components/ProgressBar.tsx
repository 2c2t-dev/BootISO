import type { FlashProgress, FlashStage } from "../types";
import "./ProgressBar.css";

interface Props {
  readonly progress: FlashProgress | null;
  readonly stage: FlashStage;
}

function ProgressBar({ progress, stage }: Props) {
  if (stage === "idle") return null;

  const percent = progress?.percent ?? 0;
  const isFlashing = stage === "flashing";
  const isDone = stage === "done";
  const isError = stage === "error";

  const formatEta = (seconds: number): string => {
    if (seconds <= 0) return "--:--";
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  };

  return (
    <div className={`progress-card card ${isDone ? "progress-done" : ""} ${isError ? "progress-error" : ""}`}>
      <div className="progress-header">
        <span className="progress-stage">
          {progress?.stage === "writing" && "Writing to USB..."}
          {progress?.stage === "verifying" && "Verifying..."}
          {progress?.stage === "done" && "Complete!"}
          {progress?.stage === "error" && "Error"}
          {!progress && stage === "ready" && "Ready to flash"}
          {!progress && stage === "done" && "Complete!"}
          {!progress && stage === "error" && "Error"}
        </span>
        {progress && isFlashing && (
          <span className="progress-speed">
            {progress.speed_mbps.toFixed(1)} MB/s
          </span>
        )}
      </div>

      <div className="progress-bar-track">
        <div
          className={`progress-bar-fill ${isFlashing ? "active" : ""} ${isDone ? "done" : ""} ${isError ? "error" : ""}`}
          style={{ width: `${Math.min(percent, 100)}%` }}
        />
      </div>

      <div className="progress-footer">
        <span className="progress-percent">{percent.toFixed(1)}%</span>
        {progress && isFlashing && (
          <span className="progress-eta">ETA: {formatEta(progress.eta_seconds)}</span>
        )}
        {progress && (
          <span className="progress-bytes">
            {formatBytes(progress.bytes_written)} / {formatBytes(progress.total_bytes)}
          </span>
        )}
      </div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  const gb = 1024 * 1024 * 1024;
  const mb = 1024 * 1024;
  if (bytes >= gb) return `${(bytes / gb).toFixed(2)} GB`;
  if (bytes >= mb) return `${(bytes / mb).toFixed(1)} MB`;
  return `${(bytes / 1024).toFixed(0)} KB`;
}

export default ProgressBar;
