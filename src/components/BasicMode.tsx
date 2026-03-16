import "./BasicMode.css";

interface Props {
  readonly step: number; // 1, 2, or 3
}

function BasicMode({ step }: Props) {
  const steps = [
    { num: 1, label: "Select ISO", icon: "💿" },
    { num: 2, label: "Select USB", icon: "🔌" },
    { num: 3, label: "Flash!", icon: "🔥" },
  ];

  return (
    <div className="basic-steps">
      {steps.map((s) => (
        <div
          key={s.num}
          className={`basic-step ${step >= s.num ? "done" : ""} ${step === s.num ? "current" : ""}`}
        >
          <div className="basic-step-num">{s.icon}</div>
          <span className="basic-step-label">{s.label}</span>
        </div>
      ))}
    </div>
  );
}

export default BasicMode;
