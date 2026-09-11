import React from 'react';

interface ToggleProps {
  id?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  disabled?: boolean;
}

const Toggle: React.FC<ToggleProps> = ({ id, checked, onChange, label, disabled = false }) => {
  return (
    <label
      id={id}
      className={`toggle ${disabled ? 'toggle--disabled' : ''}`}
      onClick={(e) => e.stopPropagation()}
    >
      <div
        className={`toggle__track ${checked ? 'toggle__track--checked' : ''}`}
        onClick={() => !disabled && onChange(!checked)}
        role="switch"
        aria-checked={checked}
        tabIndex={0}
        onKeyDown={(e) => {
          if (!disabled && (e.key === 'Enter' || e.key === ' ')) {
            e.preventDefault();
            onChange(!checked);
          }
        }}
      >
        <div className="toggle__thumb" />
      </div>
      {label && <span className="toggle__label">{label}</span>}
    </label>
  );
};

export default Toggle;
