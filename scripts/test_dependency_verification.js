const fs = require('fs')
const crypto = require('crypto')
const childProcess = require('child_process')

async function verify({
  packageLockPath = 'package-lock.json',
  markerPath = 'node_modules/.medtracker-package-lock.sha256',
  execFileSync = childProcess.execFileSync,
  launch = () => require('playwright').chromium.launch(),
  runCapybara = () => childProcess.execFileSync(
    'bundle',
    ['exec', 'ruby', 'scripts/test_capybara_browser.rb'],
    { stdio: 'inherit' }
  )
} = {}) {
  const lockHash = crypto.createHash('sha256').update(fs.readFileSync(packageLockPath)).digest('hex')
  const installedHash = fs.existsSync(markerPath) ? fs.readFileSync(markerPath, 'utf8').trim() : ''

  if (lockHash !== installedHash) {
    console.log('Refreshing test node_modules from package-lock.json')
    execFileSync('npm', ['ci', '--ignore-scripts'], { stdio: 'inherit' })
    fs.writeFileSync(markerPath, `${lockHash}\n`)
  } else {
    console.log('Test node_modules match package-lock.json')
  }

  const browser = await launch()
  await browser.close()
  runCapybara()
  console.log('Chromium launch preflight passed')
}

if (require.main === module) {
  verify().catch((error) => {
    console.error(error)
    process.exit(1)
  })
}

module.exports = { verify }
