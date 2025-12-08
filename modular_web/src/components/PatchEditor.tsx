import CodeMirror from '@uiw/react-codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { keymap } from '@codemirror/view'
import { useCallback, useEffect, useMemo, useRef } from 'react'
import { tsCompletion } from '../lsp/tsCompletion'
import { tsLinter } from '../lsp/tsDiagnostics'
import { tsHover } from '../lsp/tsHover'
import { initTsWorker, disposeTsWorker } from '../lsp/tsClient'
import type { ModuleSchema } from '../types'

interface PatchEditorProps {
  value: string
  onChange: (value: string) => void
  onSubmit: () => void
  onStop: () => void
  onSave?: () => void
  disabled?: boolean
  // Optional explicit schemas prop; currently unused but keeps the
  // MonacoPatchEditor and PatchEditor prop shapes compatible.
  schemas?: ModuleSchema[]
}

export function PatchEditor({
	  value,
	  onChange,
	  onSubmit,
	  onStop,
	  onSave,
	  disabled,
	  schemas: _schemas,
	}: PatchEditorProps) {
	  const initialValueRef = useRef(value)

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

	  useEffect(() => {
	    // Initialize the TypeScript language service worker with the initial DSL value
	    void initTsWorker('file:///modular/dsl.js', initialValueRef.current ?? '')
	    return () => {
	      disposeTsWorker()
	    }
	  }, [initialValueRef])

  const extensions = useMemo(() => {
    const exts = [
      javascript(),
      customKeymap,
	      // Frontend-only TypeScript language service features for the DSL editor
	      tsCompletion,
	      tsLinter,
	      tsHover,
    ];

    // if (schemas.length > 0) {
    //   exts.push(autocompletion({ override: [dslAutocomplete(schemas)] }));
    // }

    return exts;
  }, [customKeymap]);

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
