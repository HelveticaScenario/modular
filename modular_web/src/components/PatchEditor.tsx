import CodeMirror from '@uiw/react-codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { keymap } from '@codemirror/view'
import { autocompletion } from '@codemirror/autocomplete'
import { useCallback, useMemo } from 'react'
import type { ModuleSchema } from '../types'
import { dslAutocomplete } from '../dsl'

interface PatchEditorProps {
  value: string
  onChange: (value: string) => void
  onSubmit: () => void
  onStop: () => void
  onSave?: () => void
  disabled?: boolean
  schemas?: ModuleSchema[]
}

export function PatchEditor({
  value,
  onChange,
  onSubmit,
  onStop,
  onSave,
  disabled,
  schemas = []
}: PatchEditorProps) {
  const handleChange = useCallback((val: string) => {
    onChange(val)
  }, [onChange])

  const customKeymap = useMemo(() => keymap.of([
    {
      key: 'Ctrl-Enter',
      mac: 'Alt-Enter',
      run: () => {
        onSubmit()
        return true
      },
    },
    {
      key: 'Ctrl-.',
      mac: 'Alt-.',
      run: () => {
        onStop()
        return true
      },
    },
    {
      key: 'Ctrl-s',
      mac: 'Cmd-s',
      run: () => {
        if (onSave) {
          onSave()
        }
        return true
      },
    },
  ]), [onSubmit, onStop, onSave])

  const extensions = useMemo(() => {
    const exts = [
      javascript(),
      customKeymap,
    ];

    if (schemas.length > 0) {
      exts.push(autocompletion({ override: [dslAutocomplete(schemas)] }));
    }

    return exts;
  }, [customKeymap, schemas]);

  return (
    <div className="patch-editor">
      <CodeMirror
        value={value}
        height="100%"
        extensions={extensions}
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
