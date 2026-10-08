import { test, expect } from './care-fixtures.mjs';

test.use({ captureMail: true, registrationInviteOnly: false, actionTimeout: 10000 });

async function login(page) {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
}

async function startKey(page, origin, permissions, name = 'Synthetic scoped key') {
  const response = await page.request.post('/api/auth/security/operation/start', { headers: { Origin: origin }, data: { action: 'create_api_key', key: { name, households: [72001], permissions, expires_days: 90 } } });
  expect(response.status()).toBe(200);
  return (await response.json()).operation_id;
}

test('personal key issuance and revocation roll back with audit failure and expiry denies access', async ({ page, request: anonymous, careFixture }) => {
  await login(page);
  const headers = { Origin: careFixture.origin };
  const operation_id = await startKey(page, careFixture.origin, ['care:read']);
  await careFixture.failPersonalKeyAudit();
  const failed = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id, password: 'password' } });
  expect(failed.status()).toBe(500);
  expect((await failed.json()).apiKey).toBeUndefined();
  await careFixture.restorePersonalKeyAudit();
  const created = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id, password: 'password' } });
  expect(created.status()).toBe(200);
  const bearer = { Authorization: `Bearer ${(await created.json()).apiKey}` };
  await page.goto('/account/security/keys');
  await expect(page.getByRole('button', { name: 'Revoke API key', exact: true })).toHaveCount(1);
  const key_id = await page.locator('input[name="key_id"]').inputValue();
  await careFixture.failPersonalKeyAudit();
  expect((await page.request.post('/api/auth/security/api-key/revoke', { headers, data: { key_id } })).status()).toBe(500);
  expect((await anonymous.get(`${careFixture.origin}/api/v1/households/72001/medications`, { headers: bearer })).status()).toBe(200);
  await careFixture.restorePersonalKeyAudit();
  await careFixture.expirePersonalKeys();
  expect((await anonymous.get(`${careFixture.origin}/api/v1/households/72001/medications`, { headers: bearer })).status()).toBe(401);
});

test('care-write key cannot record doses through sync or register native push devices', async ({ page, request: anonymous, careFixture }) => {
  await login(page);
  const operation_id = await startKey(page, careFixture.origin, ['care:write']);
  const response = await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: careFixture.origin }, data: { operation_id, password: 'password' } });
  expect(response.status()).toBe(200);
  const headers = { Authorization: `Bearer ${(await response.json()).apiKey}` };
  const batch = { batch: { operations: [{ resource_type: 'medication_take', action: 'create', attributes: { client_uuid: '006b49d9-1da2-42f1-800b-8c80867aee1c', source_type: 'person_medication', source_id: '81001', taken_at: '2026-10-05T10:00:00Z', dose_amount: '2', dose_unit: 'tablet', taken_from_medication_id: 80001 } }] } };
  const before = await careFixture.probe();
  expect((await anonymous.post(`${careFixture.origin}/api/v1/households/72001/sync/batches`, { headers, data: batch })).status()).toBe(403);
  expect(await careFixture.probe()).toEqual(before);
  expect((await anonymous.post(`${careFixture.origin}/api/v1/households/72001/push_subscription`, { headers, data: {} })).status()).toBe(403);
  const both = await startKey(page, careFixture.origin, ['care:write', 'doses:write'], 'Synthetic offline integration');
  const granted = await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: careFixture.origin }, data: { operation_id: both, password: 'password' } });
  expect(granted.status()).toBe(200);
  const combined = { Authorization: `Bearer ${(await granted.json()).apiKey}` };
  expect((await anonymous.post(`${careFixture.origin}/api/v1/households/72001/sync/batches`, { headers: combined, data: batch })).status()).toBe(201);
  const after = await careFixture.probe();
  expect(after.takes).toBe(before.takes + 1);
  expect(Number(after.supply)).toBe(Number(before.supply) - 2);
});

test('personal API keys use immutable household grants and cannot become security sessions', async ({ page, request: anonymous, careFixture }) => {
  await page.goto('/login');
  await page.getByLabel('Email address', { exact: true }).fill('persistence@example.test');
  await page.getByLabel('Password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
  await page.waitForURL('/');
  await page.goto('/account/security');
  await page.getByRole('link', { name: 'Personal API keys', exact: true }).click();
  await page.getByLabel('Name', { exact: true }).fill('Synthetic read-only integration');
  await page.getByLabel('Synthetic household', { exact: true }).check();
  await page.getByLabel('Read care records', { exact: true }).check();
  await expect(page.getByLabel('Expires in days', { exact: true })).toHaveValue('90');
  await page.getByRole('button', { name: 'Create API key', exact: true }).click();
  const operation_id = await page.locator('input[name="operation_id"]').first().inputValue();
  const headers = { Origin: careFixture.origin };
  const rejectedProof = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id, password: 'incorrect' } });
  expect(rejectedProof.status()).toBe(400);
  expect(await rejectedProof.json()).toMatchObject({ code: 'INVALID_PASSWORD' });
  await expect(page.getByLabel('API key', { exact: true })).toHaveCount(0);
  await page.getByLabel('Current password', { exact: true }).fill('password');
  await page.getByRole('button', { name: 'Confirm change', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Save your API key', exact: true })).toBeVisible();
  const secret = await page.getByLabel('API key', { exact: true }).inputValue();
  expect(secret).toMatch(/^medtracker_/);
  const bearer = { Authorization: `Bearer ${secret}` };
  expect((await anonymous.get(`${careFixture.origin}/api/v1/households/72001/medications`, { headers: bearer })).status()).toBe(200);
  expect((await anonymous.get(`${careFixture.origin}/api/v1/households/72001/admin/invitations`, { headers: bearer })).status()).toBe(403);
  expect((await anonymous.get(`${careFixture.origin}/api/v1/households/72001/admin/memberships`, { headers: bearer })).status()).toBe(403);
  expect((await anonymous.post(`${careFixture.origin}/api/v1/households/72001/medications`, { headers: bearer, data: {} })).status()).toBe(403);
  expect((await anonymous.get(`${careFixture.origin}/api/v1/households/72002/medications`, { headers: bearer })).status()).toBe(403);
  expect((await anonymous.get(`${careFixture.origin}/api/v1/auth/sessions`, { headers: bearer })).status()).toBe(401);
  expect((await anonymous.post(`${careFixture.origin}/api/auth/security/operation/start`, { headers: { ...bearer, ...headers }, data: { action: 'close_account' } })).status()).toBe(401);
  expect((await anonymous.get(`${careFixture.origin}/account/security`, { headers: bearer, maxRedirects: 0 })).status()).toBe(303);
  expect((await page.request.post('/api/auth/api-key/update', { headers, data: { key: secret, permissions: { care: ['write'] } } })).status()).toBe(404);
  await page.goto('/account/security/keys');
  await expect(page.getByLabel('API key', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Synthetic read-only integration', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Revoke API key', exact: true }).click();
  await page.waitForURL('/account/security/keys');
  expect((await anonymous.get(`${careFixture.origin}/api/v1/households/72001/medications`, { headers: bearer })).status()).toBe(401);
});


test('membership withdrawal denies an issued personal key and its queued replacement', async ({ page, request: anonymous, careFixture }) => {
  await login(page);
  const headers = { Origin: careFixture.origin };
  const operation_id = await startKey(page, careFixture.origin, ['care:read']);
  const issued = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id, password: 'password' } });
  expect(issued.status()).toBe(200);
  const bearer = { Authorization: `Bearer ${(await issued.json()).apiKey}` };
  const endpoint = `${careFixture.origin}/api/v1/households/72001/medications`;
  expect((await anonymous.get(endpoint, { headers: bearer })).status()).toBe(200);
  const queued = await startKey(page, careFixture.origin, ['care:read'], 'Queued synthetic key');
  await careFixture.withdrawKeyMembership();
  expect((await anonymous.get(endpoint, { headers: bearer })).status()).toBe(403);
  const denied = await page.request.post('/api/auth/security/password/confirm', { headers, data: { operation_id: queued, password: 'password' } });
  expect(denied.status()).toBe(401);
  expect((await denied.json()).apiKey).toBeUndefined();
  await page.goto('/account/security/keys');
  await expect(page.getByRole('button', { name: 'Revoke API key', exact: true })).toHaveCount(1);
});

test('a personal key cannot authenticate after its account closes', async ({ page, request: anonymous, careFixture }) => {
  await login(page);
  const operation_id = await startKey(page, careFixture.origin, ['care:read']);
  const issued = await page.request.post('/api/auth/security/password/confirm', { headers: { Origin: careFixture.origin }, data: { operation_id, password: 'password' } });
  expect(issued.status()).toBe(200);
  const headers = { Authorization: `Bearer ${(await issued.json()).apiKey}` };
  const endpoint = `${careFixture.origin}/api/v1/households/72001/medications`;
  expect((await anonymous.get(endpoint, { headers })).status()).toBe(200);
  await careFixture.closeOtpAccount();
  expect((await anonymous.get(endpoint, { headers })).status()).toBe(401);
});
