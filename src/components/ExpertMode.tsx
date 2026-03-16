import "./ExpertMode.css";

interface Props {
  readonly filesystem: string;
  readonly setFilesystem: (v: string) => void;
  readonly partitionScheme: string;
  readonly setPartitionScheme: (v: string) => void;
  readonly volumeLabel: string;
  readonly setVolumeLabel: (v: string) => void;
  readonly bufferSize: number;
  readonly setBufferSize: (v: number) => void;
  readonly disabled: boolean;
}

function ExpertMode({
  filesystem,
  setFilesystem,
  partitionScheme,
  setPartitionScheme,
  volumeLabel,
  setVolumeLabel,
  bufferSize,
  setBufferSize,
  disabled,
}: Props) {
  return (
    <div className="card expert-mode animate-slide-up">
      <div className="card-title">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ display: 'inline', marginRight: 6, verticalAlign: 'middle' }}>
          <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
        </svg>
        Expert Options
      </div>

      <div className="form-grid">
        <div className="form-group">
          <label className="form-label" htmlFor="filesystem">File System</label>
          <select
            id="filesystem"
            className="select"
            value={filesystem}
            onChange={(e) => setFilesystem(e.target.value)}
            disabled={disabled}
          >
            <option value="auto">Auto Detect</option>
            <option value="fat32">FAT32</option>
            <option value="ntfs">NTFS</option>
            <option value="exfat">exFAT</option>
            <option value="ext4">ext4 (Linux)</option>
          </select>
        </div>

        <div className="form-group">
          <label className="form-label" htmlFor="partitionScheme">Partition Scheme</label>
          <select
            id="partitionScheme"
            className="select"
            value={partitionScheme}
            onChange={(e) => setPartitionScheme(e.target.value)}
            disabled={disabled}
          >
            <option value="mbr">MBR (Legacy/UEFI-CSM)</option>
            <option value="gpt">GPT (UEFI)</option>
          </select>
        </div>

        <div className="form-group">
          <label className="form-label" htmlFor="volumeLabel">Volume Label</label>
          <input
            id="volumeLabel"
            className="input"
            type="text"
            placeholder="BOOTISO"
            maxLength={11}
            value={volumeLabel}
            onChange={(e) => setVolumeLabel(e.target.value.toUpperCase())}
            disabled={disabled}
          />
        </div>

        <div className="form-group">
          <label className="form-label" htmlFor="bufferSize">Buffer Size</label>
          <select
            id="bufferSize"
            className="select"
            value={bufferSize}
            onChange={(e) => setBufferSize(Number(e.target.value))}
            disabled={disabled}
          >
            <option value={1}>1 MB</option>
            <option value={2}>2 MB</option>
            <option value={4}>4 MB (Default)</option>
            <option value={8}>8 MB</option>
            <option value={16}>16 MB</option>
          </select>
        </div>
      </div>
    </div>
  );
}

export default ExpertMode;
