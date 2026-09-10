const fs = require('fs')
const crypto = require('crypto')
const os = require('os')
const path = require('path')
const { verify } = require('../../scripts/test_dependency_verification')

const name = process.argv[2]
const root = fs.mkdtempSync(path.join(os.tmpdir(), 'medtracker-verifier-'))
const packageLockPath = path.join(root, 'package-lock.json')
const markerPath = path.join(root, 'node_modules', '.medtracker-package-lock.sha256')
fs.mkdirSync(path.dirname(markerPath))
fs.writeFileSync(packageLockPath, 'lock contents')
const lockHash = crypto.createHash('sha256').update(fs.readFileSync(packageLockPath)).digest('hex')
if (name === 'current' || name === 'launch_failure') fs.writeFileSync(markerPath, `${lockHash}\n`)
if (name === 'refresh_failure') fs.writeFileSync(markerPath, 'stale marker\n')

let execCalls = 0
let closed = false
const execFileSync = (command, args) => {
  execCalls += 1
  if (name === 'refresh_failure') throw new Error('refresh failed')
  if (command !== 'npm' || args.join(' ') !== 'ci --ignore-scripts') throw new Error('unexpected refresh')
}
const launch = async () => {
  if (name === 'launch_failure') throw new Error('launch failed')
  return { close: async () => { closed = true } }
}

const runCapybara = () => {}
verify({ packageLockPath, markerPath, execFileSync, launch, runCapybara }).then(() => {
  if (name === 'refresh_failure' || name === 'launch_failure') throw new Error('expected verifier failure')
  if (name === 'current' && execCalls !== 0) throw new Error('current marker refreshed')
  if (name === 'stale' && execCalls !== 1) throw new Error('stale marker not refreshed')
  if (name === 'stale' && fs.readFileSync(markerPath, 'utf8').trim() !== lockHash) throw new Error('marker not updated')
  if (name === 'refresh_failure' && fs.existsSync(markerPath)) throw new Error('failed refresh updated marker')
  if (!closed && name !== 'refresh_failure' && name !== 'launch_failure') throw new Error('browser not closed')
  console.log(`PASS ${name}`)
}).catch((error) => {
  if (name === 'refresh_failure') {
    if (error.message !== 'refresh failed') throw error
    if (fs.readFileSync(markerPath, 'utf8') !== 'stale marker\n') throw new Error('failed refresh updated marker')
    console.log(`PASS ${name}`)
    return
  }
  if (name === 'launch_failure' && error.message !== 'launch failed') throw error
  if (name !== 'launch_failure') throw error
  console.log(`PASS ${name}`)
})
