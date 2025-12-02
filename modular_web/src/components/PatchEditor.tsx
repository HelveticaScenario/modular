import CodeMirror from '@uiw/react-codemirror'
import { yaml } from '@codemirror/lang-yaml'
import { keymap } from '@codemirror/view'
import { useCallback, useMemo } from 'react'

interface PatchEditorProps {
  value: string
  onChange: (value: string) => void
  onSubmit: () => void
  onStop: () => void
  disabled?: boolean
}

export function PatchEditor({ value, onChange, onSubmit, onStop, disabled }: PatchEditorProps) {
  const handleChange = useCallback((val: string) => {
    onChange(val)
  }, [onChange])

  const customKeymap = useMemo(() => keymap.of([
    {
      key: 'Ctrl-Enter',
      mac: 'Cmd-Enter',
      run: () => {
        onSubmit()
        return true
      },
    },
    {
      key: 'Ctrl-.',
      mac: 'Cmd-.',
      run: () => {
        onStop()
        return true
      },
    },
  ]), [onSubmit, onStop])

  return (
    <div className="patch-editor">
      <CodeMirror
        value={value}
        height="100%"
        extensions={[yaml(), customKeymap]}
        onChange={handleChange}
        editable={!disabled}
        theme="dark"
        basicSetup={{
          lineNumbers: true,
          highlightActiveLine: true,
          bracketMatching: true,
          autocompletion: true,
          foldGutter: true,
        }}
      />
    </div>
  )
}
