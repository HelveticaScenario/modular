interface ErrorDisplayProps {
  error: string | null
  onDismiss: () => void
}

export function ErrorDisplay({ error, onDismiss }: ErrorDisplayProps) {
  if (!error) return null

  return (
    <div className="error-display">
      <div className="error-content">
        <span className="error-icon">⚠️</span>
        <pre className="error-message">{error}</pre>
        <button className="error-dismiss" onClick={onDismiss}>
          ×
        </button>
      </div>
    </div>
  )
}
