import type { UsbDevice } from "../types";
import "./UsbSelector.css";

interface Props {
  readonly devices: UsbDevice[];
  readonly selectedDevice: UsbDevice | null;
  readonly onDeviceSelected: (device: UsbDevice) => void;
  readonly onRefresh: () => void;
  readonly disabled: boolean;
}

function UsbSelector({
  devices,
  selectedDevice,
  onDeviceSelected,
  onRefresh,
  disabled,
}: Props) {
  return (
    <div className="card usb-selector">
      <div className="card-title-row">
        <span className="card-title">USB Device</span>
        <button
          className="btn usb-refresh-btn"
          onClick={onRefresh}
          disabled={disabled}
          title="Refresh USB devices"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="23 4 23 10 17 10" />
            <polyline points="1 20 1 14 7 14" />
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
          </svg>
          Refresh
        </button>
      </div>

      {devices.length === 0 ? (
        <div className="usb-empty">
          <div className="usb-empty-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
              <rect x="4" y="4" width="16" height="16" rx="2" ry="2" />
              <rect x="9" y="9" width="6" height="6" />
              <line x1="9" y1="1" x2="9" y2="4" />
              <line x1="15" y1="1" x2="15" y2="4" />
              <line x1="9" y1="20" x2="9" y2="23" />
              <line x1="15" y1="20" x2="15" y2="23" />
              <line x1="20" y1="9" x2="23" y2="9" />
              <line x1="20" y1="14" x2="23" y2="14" />
              <line x1="1" y1="9" x2="4" y2="9" />
              <line x1="1" y1="14" x2="4" y2="14" />
            </svg>
          </div>
          <span className="usb-empty-text">No USB devices found</span>
          <span className="usb-empty-hint">Insert a USB drive and click Refresh</span>
        </div>
      ) : (
        <div className="usb-list">
          {devices.map((device) => (
            <button
              key={device.path}
              className={`usb-device ${selectedDevice?.path === device.path ? "selected" : ""}`}
              onClick={() => onDeviceSelected(device)}
              disabled={disabled}
            >
              <div className="usb-device-icon">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M12 2v6m0 0l3-3m-3 3l-3-3" />
                  <rect x="8" y="8" width="8" height="12" rx="1" />
                  <path d="M10 12h4" />
                </svg>
              </div>
              <div className="usb-device-info">
                <span className="usb-device-name">{device.name}</span>
              </div>
              <span className="usb-device-size">{device.size_formatted}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default UsbSelector;
