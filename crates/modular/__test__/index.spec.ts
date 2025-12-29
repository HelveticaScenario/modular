import test from 'ava'

import { Synthesizer, getSchemas } from '../index'



test('Synthesizer', (t) => {
  const synth = new Synthesizer()
  synth.start()
  console.log('Synthesizer instance:', synth)
  t.pass()
})

test('getSchemas', (t) => {
  const schemas = getSchemas()
  console.log('Module Schemas:', schemas)
  t.true(Array.isArray(schemas))
})