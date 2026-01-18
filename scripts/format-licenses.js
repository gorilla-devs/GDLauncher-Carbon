#!/usr/bin/env node
// Usage: pnpm licenses list --json --recursive | node scripts/format-licenses.js > THIRD-PARTY-NPM.txt

const LICENSE_URLS = {
  'MIT': 'https://opensource.org/licenses/MIT',
  'Apache-2.0': 'https://www.apache.org/licenses/LICENSE-2.0',
  'BSD-2-Clause': 'https://opensource.org/licenses/BSD-2-Clause',
  'BSD-3-Clause': 'https://opensource.org/licenses/BSD-3-Clause',
  'ISC': 'https://opensource.org/licenses/ISC',
  'MPL-2.0': 'https://www.mozilla.org/en-US/MPL/2.0/',
  'Zlib': 'https://opensource.org/licenses/Zlib',
  'CC0-1.0': 'https://creativecommons.org/publicdomain/zero/1.0/',
  'CC-BY-4.0': 'https://creativecommons.org/licenses/by/4.0/',
  'CC-BY-3.0': 'https://creativecommons.org/licenses/by/3.0/',
  'Unlicense': 'https://unlicense.org/',
  '0BSD': 'https://opensource.org/licenses/0BSD',
  'BlueOak-1.0.0': 'https://blueoakcouncil.org/license/1.0.0',
  'Python-2.0': 'https://www.python.org/download/releases/2.0/license/',
};

let data = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => data += chunk);
process.stdin.on('end', () => {
  const licenses = JSON.parse(data);

  // Collect all packages with their licenses
  const allPackages = [];
  for (const [license, packages] of Object.entries(licenses)) {
    for (const p of packages) {
      const name = p.name || p.package || 'unknown';
      allPackages.push({ name, license });
    }
  }

  // Dedupe and sort by name
  const unique = [...new Map(allPackages.map(p => [p.name, p])).values()]
    .sort((a, b) => a.name.localeCompare(b.name));

  console.log('THIRD-PARTY NPM PACKAGES');
  console.log('========================\n');

  for (const pkg of unique) {
    console.log(`- ${pkg.name} [${pkg.license}]`);
  }

  console.log(`\n================================================================================`);
  console.log(`Total: ${unique.length} packages`);
  console.log(`================================================================================\n`);

  console.log('LICENSE TEXTS');
  console.log('=============\n');

  const usedLicenses = [...new Set(allPackages.map(p => p.license))].sort();
  for (const license of usedLicenses) {
    const url = LICENSE_URLS[license] || `https://spdx.org/licenses/${license}.html`;
    console.log(`${license}: ${url}`);
  }

  console.log(`\n================================================================================`);
  console.log('Full license texts are available at the URLs above or at https://spdx.org/licenses/');
});
