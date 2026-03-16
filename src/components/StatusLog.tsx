import "./StatusLog.css";

interface Props {
  readonly logs: string[];
  readonly isOpen: boolean;
  readonly onClose: () => void;
}

function StatusLog({ logs, isOpen, onClose }: Props) {
  return (
    <div className={`status-log card ${isOpen ? "open" : ""}`}>
      <div className="card-title log-title">
        <div className="log-title-text">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          Activity Log
        </div>
        <button className="log-close-btn" onClick={onClose} title="Close Logs">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>
      <div className="log-container">
        {logs.length === 0 ? (
          <div className="log-empty">
            <span className="log-empty-text">Waiting for activity...</span>
          </div>
        ) : (
          logs.map((log, i) => (
            // eslint-disable-next-line react/no-array-index-key
            <div key={i} className={`log-entry ${i === 0 ? "log-latest" : ""}`}>
              {log}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

export default StatusLog;
