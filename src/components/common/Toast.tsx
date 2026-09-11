import React from 'react';
import { useUIStore } from '../../store/uiStore';
import type { Toast as ToastType } from '../../store/uiStore';

const iconMap: Record<string, string> = {
  success: '✓',
  error: '✕',
  warning: '⚠',
  info: 'ℹ',
};

const ToastItem: React.FC<{ toast: ToastType }> = ({ toast }) => {
  const removeToast = useUIStore((s) => s.removeToast);

  return (
    <div className={`toast toast--${toast.type}`} role="alert">
      <div className="toast__icon">{iconMap[toast.type]}</div>
      <div className="toast__content">
        <div className="toast__title">{toast.title}</div>
        {toast.message && <div className="toast__message">{toast.message}</div>}
      </div>
      <button
        className="toast__close"
        onClick={() => removeToast(toast.id)}
        aria-label="Close notification"
      >
        ✕
      </button>
    </div>
  );
};

const ToastContainer: React.FC = () => {
  const toasts = useUIStore((s) => s.toasts);

  if (toasts.length === 0) return null;

  return (
    <div className="toast-container" id="toast-container">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>
  );
};

export default ToastContainer;
