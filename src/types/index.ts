export interface UsbDevice {
  name: string;
  path: string;
  size: number;
  size_formatted: string;
  vendor: string;
  model: string;
  removable: boolean;
}

export type IsoType = "Windows" | "Linux" | "Unknown";

export interface IsoInfo {
  path: string;
  filename: string;
  size: number;
  size_formatted: string;
  iso_type: IsoType;
  label: string;
  valid: boolean;
  sha256: string | null;
  boot_type: string;
}

export interface FlashOptions {
  iso_path: string;
  device_path: string;
  buffer_size?: number;
  verify_after_write?: boolean;
  filesystem?: string;
  partition_scheme?: string;
  volume_label?: string;
}

export interface FlashProgress {
  bytes_written: number;
  total_bytes: number;
  percent: number;
  speed_mbps: number;
  eta_seconds: number;
  stage: "writing" | "verifying" | "done" | "error";
}

export interface FlashResult {
  success: boolean;
  message: string;
  duration_seconds: number;
  bytes_written: number;
  verified: boolean;
}

export interface PlatformInfo {
  os: string;
  arch: string;
  is_admin: boolean;
}

export type AppMode = "basic" | "expert";
export type FlashStage = "idle" | "ready" | "flashing" | "writing" | "verifying" | "done" | "error";
