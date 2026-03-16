import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { IsoInfo } from "../types";
import "./IsoSelector.css";

interface Props {
  readonly iso: IsoInfo | null;
  readonly onIsoSelected: (info: IsoInfo) => void;
  readonly disabled: boolean;
}

function IsoSelector({ iso, onIsoSelected, disabled }: Props) {
  const [isDragOver, setIsDragOver] = useState(false);
  const [loading, setLoading] = useState(false);


  const handleBrowse = useCallback(async () => {
    if (disabled) return;

    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "ISO Image", extensions: ["iso"] }],
      });

      if (selected) {
        setLoading(true);
        const info = await invoke<IsoInfo>("validate_iso", { path: selected });
        onIsoSelected(info);
        setLoading(false);
      }
    } catch (e) {
      console.error("ISO browse error:", e);
      setLoading(false);
    }
  }, [disabled, onIsoSelected]);

  const handleDragOver = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      if (!disabled) setIsDragOver(true);
    },
    [disabled]
  );

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);
      if (disabled) return;

      const files = e.dataTransfer.files;
      if (files.length > 0) {
        const file = files[0];
        if (file.name.toLowerCase().endsWith(".iso")) {
          setLoading(true);
          try {
            const info = await invoke<IsoInfo>("validate_iso", {
              path: (file as unknown as { path?: string }).path || file.name,
            });
            onIsoSelected(info);
          } catch (err) {
            console.error("Drop error:", err);
          }
          setLoading(false);
        }
      }
    },
    [disabled, onIsoSelected]
  );

  const getIsoTypeBadge = () => {
    if (!iso) return null;
    switch (iso.iso_type) {
      case "Windows":
        return <span className="badge badge-blue">Windows</span>;
      case "Linux":
        return <span className="badge badge-orange">Linux</span>;
      default:
        return <span className="badge badge-violet">ISO</span>;
    }
  };

  return (
    <div className="card iso-selector">
      <div className="card-title">ISO Image</div>

      {iso ? (
        <div className="iso-info">
          <div className="iso-info-header">
            <div className="iso-info-name" title={iso.filename}>
              {iso.filename}
            </div>
            <button
              className="btn iso-change-btn"
              onClick={handleBrowse}
              disabled={disabled}
            >
              Change
            </button>
          </div>
          <div className="iso-info-details">
            <div className="iso-detail">
              <span className="iso-detail-label">Size</span>
              <span className="iso-detail-value">{iso.size_formatted}</span>
            </div>
            <div className="iso-detail">
              <span className="iso-detail-label">Type</span>
              {getIsoTypeBadge()}
            </div>
          </div>
          {iso.valid ? (
            <div className="iso-valid">
              <span className="badge badge-green">Valid ISO 9660</span>
            </div>
          ) : (
            <div className="iso-valid">
              <span className="badge badge-red">Invalid ISO</span>
            </div>
          )}
        </div>
      ) : (
        <button
          type="button"
          className={`iso-dropzone ${isDragOver ? "drag-over" : ""} ${loading ? "loading" : ""}`}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          onClick={handleBrowse}
        >
          {loading ? (
            <div className="iso-loading">
              <div className="spinner" />
              <span>Validating ISO...</span>
            </div>
          ) : (
            <>
              <div className="iso-dropzone-icon">
                <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="12" cy="12" r="10" />
                  <circle cx="12" cy="12" r="3" />
                  <line x1="12" y1="2" x2="12" y2="5" />
                  <line x1="12" y1="19" x2="12" y2="22" />
                  <line x1="2" y1="12" x2="5" y2="12" />
                  <line x1="19" y1="12" x2="22" y2="12" />
                </svg>
              </div>
              <span className="iso-dropzone-text">
                Drop ISO here or <strong>click to browse</strong>
              </span>
              <span className="iso-dropzone-hint">.iso files only</span>
            </>
          )}
        </button>
      )}
    </div>
  );
}

export default IsoSelector;
