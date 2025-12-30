import test from 'ava'

import { getSchemas } from '../index'



// test('Synthesizer', (t) => {
//   const synth = new Synthesizer()
//   synth.start()
//   console.log('Synthesizer instance:', synth.getScopes())
//   t.pass()
// })

test('getSchemas', (t) => {
  const schemas = getSchemas()
  console.log('Module Schemas:', schemas)
  t.true(Array.isArray(schemas))
})