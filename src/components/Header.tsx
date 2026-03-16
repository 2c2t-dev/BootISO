import type { PlatformInfo } from "../types";
import "./Header.css";

interface Props {
  readonly platform: PlatformInfo | null;
}

function Header({ platform }: Props) {
  return (
    <header className="header">
      <div className="header-left">
        <div className="header-logo">
          <div className="header-logo-icon">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
              <polyline points="7.5 4.21 12 6.81 16.5 4.21" />
              <polyline points="7.5 19.79 7.5 14.6 3 12" />
              <polyline points="21 12 16.5 14.6 16.5 19.79" />
              <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
              <line x1="12" y1="22.08" x2="12" y2="12" />
            </svg>
          </div>
          <div className="header-title-group">
            <h1 className="header-title">BootISO</h1>
            <span className="header-version">v0.1.0</span>
          </div>
        </div>
      </div>

      <div className="header-right">
        {platform && (
          <div className="header-platform">
            <span className="header-os">{capitalizeOS(platform.os)}</span>
            <span className="header-divider">·</span>
            <span className="header-arch">{platform.arch}</span>
            {!platform.is_admin && (
              <>
                <span className="header-divider">·</span>
                <span className="header-warn" title="Run as administrator for USB write access">
                  ⚠️ Non-admin
                </span>
              </>
            )}
          </div>
        )}
      </div>
    </header>
  );
}


function capitalizeOS(os: string): string {
  switch (os) {
    case "windows": return "Windows";
    case "macos": return "macOS";
    case "linux": return "Linux";
    default: return os;
  }
}

export default Header;
