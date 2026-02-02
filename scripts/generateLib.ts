import { getSchemas } from '@modular/core';
import { buildLibSource } from '../src/dsl/typescriptLibGen';
import * as fs from 'fs';
import * as path from 'path';

async function main() {
  console.log('Fetching schemas...');
  const schemas = getSchemas();
  console.log(`Found ${schemas.length} module schemas`);
  
  console.log('Building library source...');
  const libSource = buildLibSource(schemas);
  
  const outputPath = path.join(__dirname, '../generated/dsl.d.ts');
  const outputDir = path.dirname(outputPath);
  
  // Ensure output directory exists
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }
  
  fs.writeFileSync(outputPath, libSource, 'utf-8');
  console.log(`✓ Library source written to ${outputPath}`);
  console.log(`  Size: ${Math.round(libSource.length / 1024)} KB`);
}

main().catch((error) => {
  console.error('Error:', error);
  process.exit(1);
});
