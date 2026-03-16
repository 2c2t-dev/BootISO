import { useState } from "react";
import type { FlashStage, IsoInfo, UsbDevice } from "../types";
import "./FlashButton.css";

interface Props {
  readonly stage: FlashStage;
  readonly iso: IsoInfo | null;
  readonly device: UsbDevice | null;
  readonly onFlash: () => void;
  readonly onCancel: () => void;
  readonly onSkip: () => void;
  readonly onReset: () => void;
}

function FlashButton({ stage, iso, device, onFlash, onCancel, onSkip, onReset }: Props) {
  const [showConfirm, setShowConfirm] = useState(false);

  const canFlash = iso && device && iso.valid && stage === "ready";
  const isFlashing = stage === "flashing" || stage === "writing";
  const isVerifying = stage === "verifying";
  const isDone = stage === "done";
  const isError = stage === "error";

  const handleFlashClick = () => {
    if (canFlash) {
      setShowConfirm(true);
    }
  };

  const handleConfirm = () => {
    setShowConfirm(false);
    onFlash();
  };

  const handleCancelConfirm = () => {
    setShowConfirm(false);
  };

  if (showConfirm) {
    return (
      <div className="flash-confirm card animate-in">
        <div className="flash-confirm-icon">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
            <line x1="12" y1="9" x2="12" y2="13"/>
            <line x1="12" y1="17" x2="12.01" y2="17"/>
          </svg>
        </div>
        <h3 className="flash-confirm-title">Confirm Flash</h3>
        <p className="flash-confirm-text">
          This will <strong>erase all data</strong> on:
        </p>
        <div className="flash-confirm-device">
          <strong>{device?.name}</strong>
          <span className="flash-confirm-path">{device?.path}</span>
        </div>
        <p className="flash-confirm-text">
          Writing: <strong>{iso?.filename}</strong>
        </p>
        <div className="flash-confirm-actions">
          <button className="btn" onClick={handleCancelConfirm}>
            Cancel
          </button>
          <button className="btn btn-danger flash-confirm-btn" onClick={handleConfirm}>
            Yes, Flash Now
          </button>
        </div>
      </div>
    );
  }

  if (isDone || isError) {
    return (
      <button className={`flash-btn btn ${isDone ? "btn-success" : "btn-danger"}`} onClick={onReset}>
        {isDone ? "Done — Flash Another" : "Error — Try Again"}
      </button>
    );
  }

  if (isFlashing) {
    return (
      <button className="flash-btn btn btn-danger" onClick={onCancel}>
        Cancel Flash
      </button>
    );
  }

  if (isVerifying) {
    return (
      <button className="flash-btn btn btn-secondary" onClick={onSkip}>
        Skip Verification
      </button>
    );
  }

  let buttonText = "Flash ISO to USB";
  if (!iso) {
    buttonText = "Select an ISO first";
  } else if (!device) {
    buttonText = "Select a USB device";
  }

  return (
    <button
      className="flash-btn btn btn-primary"
      onClick={handleFlashClick}
      disabled={!canFlash}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
      </svg>
      {buttonText}
    </button>
  );
}

export default FlashButton;
