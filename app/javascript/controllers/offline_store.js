const DB_NAME = "medtracker-offline"
const DB_VERSION = 1
const SNAPSHOT_KEY = "snapshot"
const DEFAULT_TENANT_KEY = "global"

function openDatabase() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION)

    request.onupgradeneeded = () => {
      const db = request.result
      if (!db.objectStoreNames.contains("keyval")) db.createObjectStore("keyval")
      if (!db.objectStoreNames.contains("queuedTakes")) db.createObjectStore("queuedTakes", { keyPath: "client_uuid" })
      if (!db.objectStoreNames.contains("failedTakes")) db.createObjectStore("failedTakes", { keyPath: "client_uuid" })
    }

    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  })
}

async function transaction(storeName, mode, callback) {
  const db = await openDatabase()

  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, mode)
    const store = tx.objectStore(storeName)
    const result = callback(store)

    tx.oncomplete = () => resolve(result)
    tx.onerror = () => reject(tx.error)
    tx.onabort = () => reject(tx.error)
  })
}

function requestResult(request) {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  })
}

export async function getValue(key) {
  const db = await openDatabase()
  const tx = db.transaction("keyval", "readonly")
  return requestResult(tx.objectStore("keyval").get(key))
}

export async function setValue(key, value) {
  return transaction("keyval", "readwrite", (store) => store.put(value, key))
}

export function defaultTenantKey() {
  const householdId = metaContent("med-tracker-household-id")
  const membershipId = metaContent("med-tracker-membership-id")

  if (householdId && membershipId) return `household:${householdId}:membership:${membershipId}`
  if (householdId) return `household:${householdId}`

  return DEFAULT_TENANT_KEY
}

function metaContent(name) {
  return document.querySelector(`meta[name='${name}']`)?.content || ""
}

function normalizedTenantKey(tenantKey = defaultTenantKey()) {
  return tenantKey || DEFAULT_TENANT_KEY
}

function snapshotKey(tenantKey) {
  return `${SNAPSHOT_KEY}:${normalizedTenantKey(tenantKey)}`
}

export async function getSnapshot(tenantKey = defaultTenantKey()) {
  return getValue(snapshotKey(tenantKey))
}

export async function saveSnapshot(payload, tenantKey = defaultTenantKey()) {
  return setValue(snapshotKey(tenantKey), {
    cached_at: new Date().toISOString(),
    payload
  })
}

export async function refreshSnapshot(snapshotUrl, tenantKey = defaultTenantKey()) {
  const response = await fetch(snapshotUrl, {
    credentials: "same-origin",
    headers: { Accept: "application/json" }
  })

  if (!response.ok) throw new Error(`Snapshot failed with ${response.status}`)

  const payload = await response.json()
  await saveSnapshot(payload, tenantKey)
  return getSnapshot(tenantKey)
}

export function buildClientUuid() {
  if (window.crypto?.randomUUID) return window.crypto.randomUUID()

  return `offline-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

export async function queueTake(attributes, tenantKey = defaultTenantKey()) {
  tenantKey = normalizedTenantKey(tenantKey)
  const clientUuid = attributes.client_uuid || buildClientUuid()
  const take = {
    ...attributes,
    household_key: tenantKey,
    client_uuid: clientUuid,
    queued_at: attributes.queued_at || new Date().toISOString(),
    attempts: attributes.attempts || 0
  }

  await transaction("queuedTakes", "readwrite", (store) => store.put(take))
  return take
}

export async function getQueuedTakes(tenantKey = defaultTenantKey()) {
  tenantKey = normalizedTenantKey(tenantKey)
  const db = await openDatabase()
  const tx = db.transaction("queuedTakes", "readonly")
  const takes = await requestResult(tx.objectStore("queuedTakes").getAll())
  return takes.filter((take) => take.household_key === tenantKey)
}

export async function removeQueuedTake(clientUuid) {
  return transaction("queuedTakes", "readwrite", (store) => store.delete(clientUuid))
}

export async function getFailedTakes(tenantKey = defaultTenantKey()) {
  tenantKey = normalizedTenantKey(tenantKey)
  const db = await openDatabase()
  const tx = db.transaction("failedTakes", "readonly")
  const takes = await requestResult(tx.objectStore("failedTakes").getAll())
  return takes.filter((take) => take.household_key === tenantKey)
}

export async function saveFailedTake(take, message, tenantKey = take.household_key || defaultTenantKey()) {
  tenantKey = normalizedTenantKey(tenantKey)
  return transaction("failedTakes", "readwrite", (store) => {
    store.put({
      ...take,
      household_key: tenantKey,
      failed_at: new Date().toISOString(),
      failure_message: message
    })
  })
}

export async function syncQueuedTakes(syncUrl, tenantKey = defaultTenantKey()) {
  tenantKey = normalizedTenantKey(tenantKey)
  const queued = await getQueuedTakes(tenantKey)
  const csrfToken = document.querySelector("meta[name='csrf-token']")?.content
  const result = { synced: [], failed: [], authRequired: false }

  for (const take of queued) {
    const response = await fetch(syncUrl, {
      method: "POST",
      credentials: "same-origin",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
        ...(csrfToken ? { "X-CSRF-Token": csrfToken } : {})
      },
      body: JSON.stringify({ ...take, household_key: undefined, attempts: undefined, queued_at: undefined })
    })

    const contentType = response.headers.get("content-type") || ""
    if (response.status === 401 || response.status === 403 || !contentType.includes("application/json")) {
      result.authRequired = true
      break
    }

    const payload = await response.json()
    if (response.ok) {
      await removeQueuedTake(take.client_uuid)
      result.synced.push(payload.data)
    } else {
      const message = payload.error?.message || "Sync failed"
      await removeQueuedTake(take.client_uuid)
      await saveFailedTake(take, message, tenantKey)
      result.failed.push({ take, message })
    }
  }

  return result
}
