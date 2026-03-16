import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import IsoSelector from "./components/IsoSelector";
import UsbSelector from "./components/UsbSelector";
import ModeSwitch from "./components/ModeSwitch";

import ExpertMode from "./components/ExpertMode";
import ProgressBar from "./components/ProgressBar";
import FlashButton from "./components/FlashButton";
import StatusLog from "./components/StatusLog";
import type {
  AppMode,
  FlashStage,
  IsoInfo,
  UsbDevice,
  FlashProgress,
  FlashOptions,
  FlashResult,
} from "./types";
import "./App.css";

function App() {
  const [mode, setMode] = useState<AppMode>("basic");
  const [stage, setStage] = useState<FlashStage>("idle");
  const [iso, setIso] = useState<IsoInfo | null>(null);
  const [devices, setDevices] = useState<UsbDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<UsbDevice | null>(null);
  const [progress, setProgress] = useState<FlashProgress | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [showLogs, setShowLogs] = useState(false);
  const showLogsRef = useRef(false);
  const [unreadLogs, setUnreadLogs] = useState(0);

  useEffect(() => {
    showLogsRef.current = showLogs;
  }, [showLogs]);

  // Expert mode options
  const [filesystem, setFilesystem] = useState("auto");
  const [partitionScheme, setPartitionScheme] = useState("mbr");
  const [volumeLabel, setVolumeLabel] = useState("");
  const [bufferSize, setBufferSize] = useState(4);

  const addLog = useCallback((msg: string) => {
    const time = new Date().toLocaleTimeString();
    setLogs((prev) => [`[${time}] ${msg}`, ...prev].slice(0, 200));
    if (!showLogsRef.current) {
      setUnreadLogs((prev) => prev + 1);
    }
  }, []);



  // Listen for flash progress events
  useEffect(() => {
    const unlisten = listen<FlashProgress>("flash-progress", (event) => {
      setProgress(event.payload);
      const stage = event.payload.stage;
      
      if (stage === "done") {
        setStage("done");
      } else if (stage === "verifying") {
        setStage("verifying");
      } else if (stage === "writing") {
        setStage("writing");
      } else if (stage === "error") {
        setStage("error");
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addLog]);

  const refreshDevices = useCallback(async () => {
    try {
      addLog("Scanning USB devices...");
      const devs = await invoke<UsbDevice[]>("list_usb_devices");
      setDevices(devs);
      addLog(`Found ${devs.length} USB device(s)`);
      if (devs.length > 0 && !selectedDevice) {
        setSelectedDevice(devs[0]);
      }
    } catch (e) {
      addLog(`USB scan error: ${e}`);
    }
  }, [addLog, selectedDevice]);

  // Auto-refresh devices on mount
  useEffect(() => {
    refreshDevices();
  }, []);

  const handleIsoSelected = useCallback(
    (info: IsoInfo) => {
      setIso(info);
      addLog(`ISO loaded: ${info.filename} (${info.size_formatted})`);
      addLog(`Type: ${info.iso_type} | Boot: ${info.boot_type}`);
      addLog(`Label: ${info.label}`);
      if (selectedDevice) {
        setStage("ready");
      }
    },
    [addLog, selectedDevice]
  );

  const handleDeviceSelected = useCallback(
    (device: UsbDevice) => {
      setSelectedDevice(device);
      addLog(`Selected: ${device.name} (${device.size_formatted})`);
      if (iso) {
        setStage("ready");
      }
    },
    [addLog, iso]
  );

  const handleFlash = useCallback(async () => {
    if (!iso || !selectedDevice) return;

    setStage("flashing");
    setProgress(null);
    addLog("Starting flash...");
    addLog(`ISO: ${iso.filename}`);
    addLog(`Device: ${selectedDevice.path}`);

    const options: FlashOptions = {
      iso_path: iso.path,
      device_path: selectedDevice.path,
      buffer_size: bufferSize * 1024 * 1024,
      filesystem: mode === "expert" ? filesystem : "auto",
      partition_scheme: mode === "expert" ? partitionScheme : "mbr",
      volume_label: volumeLabel || undefined,
    };

    try {
      const result = await invoke<FlashResult>("start_flash", { options });
      if (result.success) {
        setStage("done");
        addLog(result.message);
        if (result.verified) addLog("Verification passed");
      } else {
        setStage("error");
        addLog(`Warning: ${result.message}`);
      }
    } catch (e) {
      setStage("error");
      addLog(`Flash error: ${e}`);
    }
  }, [
    iso,
    selectedDevice,
    mode,
    filesystem,
    partitionScheme,
    volumeLabel,
    bufferSize,
    addLog,
  ]);

  const handleCancel = useCallback(async () => {
    try {
      await invoke("cancel_flash");
      addLog("Flash cancelled");
      setStage("ready");
    } catch (e) {
      addLog(`Cancel error: ${e}`);
    }
  }, [addLog]);

  const handleSkipVerification = useCallback(async () => {
    if (stage === "verifying") {
      try {
        await invoke("cancel_flash");
        addLog("Verification skipped");
        setStage("done");
      } catch (e) {
        addLog(`Skip error: ${e}`);
      }
    }
  }, [stage, addLog]);

  const handleReset = useCallback(() => {
    setStage("idle");
    setIso(null);
    setProgress(null);
    addLog("Reset — ready for a new flash");
  }, [addLog]);

  const isFlashing = stage === "flashing";

  const handleToggleLogs = useCallback(() => {
    setShowLogs((prev) => {
      if (!prev) setUnreadLogs(0);
      return !prev;
    });
  }, []);

  return (
    <div className="app">
      <main className="app-main">
        <div className="app-content">
          <ModeSwitch mode={mode} onChange={setMode} disabled={isFlashing} />

            <IsoSelector
              iso={iso}
              onIsoSelected={handleIsoSelected}
              disabled={isFlashing}
            />

            <UsbSelector
              devices={devices}
              selectedDevice={selectedDevice}
              onDeviceSelected={handleDeviceSelected}
              onRefresh={refreshDevices}
              disabled={isFlashing}
            />

            {mode === "expert" && (
              <ExpertMode
                filesystem={filesystem}
                setFilesystem={setFilesystem}
                partitionScheme={partitionScheme}
                setPartitionScheme={setPartitionScheme}
                volumeLabel={volumeLabel}
                setVolumeLabel={setVolumeLabel}
                bufferSize={bufferSize}
                setBufferSize={setBufferSize}
                disabled={isFlashing}
              />
            )}

            <ProgressBar progress={progress} stage={stage} />

            <FlashButton
              stage={stage}
              iso={iso}
              device={selectedDevice}
              onFlash={handleFlash}
              onCancel={handleCancel}
              onSkip={handleSkipVerification}
              onReset={handleReset}
            />
        </div>
      </main>

      <footer className="app-footer">
        <button
          className="footer-link"
          onClick={(e) => {
            e.preventDefault();
            // Will open the user's web platform
            window.open("https://bootiso.app", "_blank");
          }}
          title="Download ISOs from web platform"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
          Get ISO
        </button>

        <button
          className="footer-btn"
          onClick={handleToggleLogs}
          title="Toggle Activity Log"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          Logs
          {unreadLogs > 0 && (
            <span className="footer-badge">
              {unreadLogs > 99 ? '99+' : unreadLogs}
            </span>
          )}
        </button>
      </footer>

      <StatusLog 
        logs={logs} 
        isOpen={showLogs} 
        onClose={() => setShowLogs(false)} 
      />
    </div>
  );
}

export default App;
