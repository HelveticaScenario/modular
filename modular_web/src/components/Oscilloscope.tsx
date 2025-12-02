import { useEffect, useRef } from 'react'

interface OscilloscopeProps {
  data: number[] | null
  width?: number
  height?: number
}

export function Oscilloscope({ data, width = 800, height = 200 }: OscilloscopeProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    // Clear canvas
    ctx.fillStyle = '#1a1a1a'
    ctx.fillRect(0, 0, width, height)

    // Draw center line
    ctx.strokeStyle = '#333'
    ctx.lineWidth = 1
    ctx.beginPath()
    ctx.moveTo(0, height / 2)
    ctx.lineTo(width, height / 2)
    ctx.stroke()

    if (!data || data.length === 0) {
      // Draw "No Signal" text
      ctx.fillStyle = '#666'
      ctx.font = '14px monospace'
      ctx.textAlign = 'center'
      ctx.fillText('No Signal', width / 2, height / 2)
      return
    }

    // Draw waveform
    ctx.strokeStyle = '#00ff00'
    ctx.lineWidth = 2
    ctx.beginPath()

    const step = width / data.length
    const midY = height / 2
    const amplitude = height / 2 - 10

    for (let i = 0; i < data.length; i++) {
      const x = i * step
      const y = midY - data[i] * amplitude

      if (i === 0) {
        ctx.moveTo(x, y)
      } else {
        ctx.lineTo(x, y)
      }
    }

    ctx.stroke()
  }, [data, width, height])

  return (
    <div className="oscilloscope">
      <canvas
        ref={canvasRef}
        width={width}
        height={height}
        style={{ width: '100%', height: 'auto', maxWidth: width }}
      />
    </div>
  )
}
