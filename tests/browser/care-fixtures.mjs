import { test as base, expect } from '@playwright/test';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { createServer } from 'node:net';
import { fileURLToPath } from 'node:url';
import { randomUUID } from 'node:crypto';
import { mkdtemp, rm, mkdir, writeFile, cp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { withOwnedDatabase } from '../../scripts/migration/foundation-database.mjs';
import { stopOwnedProcess } from '../../scripts/migration/process-cleanup.mjs';

const execute = promisify(execFile);
const root = fileURLToPath(new URL('../../', import.meta.url));

async function withCareFixture(captureMail, registrationInviteOnly, reportAssetsUnavailable, use, oidcProvider = false) {
  const runtimeTimings = { phases: [] };
  const measure = async (name, action) => {
    const started = performance.now();
    try { return await action(); } finally { runtimeTimings.phases.push({ name, durationMs: performance.now() - started }); }
  };
  const databaseStarted = performance.now();
  let databaseOwnership;
  try { await withOwnedDatabase(async (databaseUrl, provision, ownership) => {
    databaseOwnership = ownership;
    runtimeTimings.phases.push({ name: 'database', durationMs: performance.now() - databaseStarted });
    await measure('provision', provision);
    const owner = new URL(databaseUrl);
    owner.pathname = '/medtracker_reference';
    if (owner.hostname !== '127.0.0.1') throw Error('Care fixture must use the owned loopback database');
    await measure('migration', () => execute('task', ['browser-care:migrate', `MIGRATION_DATABASE_URL=${owner.href}`, 'LOCO_ENV=test'], { cwd: root, timeout: 60000 }));
    const fixtureTask = async (name, variables = []) => (await execute('task', [`browser-care:${name}`, `CARE_DATABASE_URL=${owner.href}`, `FOUNDATION_PROJECT=${ownership.project}`, ...variables], { cwd: root, timeout: 60000 })).stdout.trim();
    await measure('seed', () => fixtureTask('seed'));
    const snapshotDirectory = await mkdtemp(join(tmpdir(), 'mtloco-browser-'));
    const snapshotPath = join(snapshotDirectory, 'baseline.dump');
    let applicationRoot = root;
    let applicationBinary;
    if (reportAssetsUnavailable) {
      applicationRoot = join(snapshotDirectory, 'application');
      await mkdir(join(applicationRoot, 'assets'), { recursive: true });
      await Promise.all(['config', 'assets/views', 'assets/static', 'assets/reports'].map(async path => {
        const destination = join(applicationRoot, path);
        await mkdir(join(destination, '..'), { recursive: true });
        await cp(join(root, path), destination, { recursive: true });
      }));
      await rm(join(applicationRoot, 'assets/reports/fonts'), { recursive: true, force: true });
      applicationBinary = (await execute('task', ['browser-care:binary'], { cwd: root, timeout: 30000 })).stdout.trim();
    }
    const runtime = new URL(owner);
    runtime.username = 'medtracker_browser_runtime';
    runtime.password = 'password';
    const reservation = createServer();
    await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
    const port = reservation.address().port;
    const origin = `http://localhost:${port}`;
    const mailName = `mtloco-mail-${randomUUID()}`;
    let mailStarted = false;
    let mailpitUrl;
    let smtpPort;
    let server;
    let provider;
    let output = '';
    let exited = false;
    const start = async () => {
      output = '';
      exited = false;
      const environment = { ...process.env, LOCO_ENV: 'test', TZ: 'UTC', BINDING: '127.0.0.1', PORT: String(port), MEDTRACKER_PUBLIC_HOST: 'http://localhost', DATABASE_URL: runtime.href, MEDTRACKER_CAPTURE_MAIL: String(captureMail), MEDTRACKER_SMTP_PORT: smtpPort ?? '1025', MEDTRACKER_SESSION_KEY: Buffer.alloc(64, 7).toString('base64'), MEDTRACKER_COOKIE_SECURE: 'false', RAILS_SECRET_KEY_BASE: 'synthetic-rails-secret-key-base-for-compatibility', RAILS_OLD_SECRET_KEY_BASE: 'synthetic-old-rails-secret-key-base', AUTH_SESSION_SECRET: 'synthetic-browser-occurrence-signing-key-32' };
      for (const name of ['MEDTRACKER_ZITADEL_ISSUER', 'MEDTRACKER_ZITADEL_CLIENT_ID', 'MEDTRACKER_ZITADEL_CLIENT_SECRET']) delete environment[name];
      if (provider) Object.assign(environment, { MEDTRACKER_ZITADEL_ISSUER: provider.origin, MEDTRACKER_ZITADEL_CLIENT_ID: 'medtracker-fixture', MEDTRACKER_ZITADEL_CLIENT_SECRET: 'password' });
      delete environment.RUST_LOG;
      delete environment.INVITE_ONLY;
      if (registrationInviteOnly !== undefined) environment.INVITE_ONLY = String(registrationInviteOnly);
      if (reservation.listening) await new Promise(resolve => reservation.close(resolve));
      server = reportAssetsUnavailable
        ? spawn(applicationBinary, ['start', '--environment', 'test'], { cwd: applicationRoot, detached: true, env: environment, stdio: ['ignore', 'pipe', 'pipe'] })
        : spawn('task', ['browser-care:serve', ...(captureMail ? ['SERVER_AND_WORKER=true'] : [])], { cwd: root, detached: true, env: environment, stdio: ['ignore', 'pipe', 'pipe'] });
      server.stdout.on('data', chunk => { output += chunk; });
      server.stderr.on('data', chunk => { output += chunk; });
      server.on('exit', () => { exited = true; });
      server.on('error', error => { exited = true; output += error.message; });
      const deadline = Date.now() + 60000;
      let ready = false;
      while (Date.now() < deadline) {
        if (exited) throw Error(`Owned care server stopped: ${output}`);
        try { ready = (await fetch(`${origin}/up`, { signal: AbortSignal.timeout(1000) })).ok; } catch {}
        if (ready) break;
        await new Promise(resolve => setTimeout(resolve, 250));
      }
      if (!ready) throw Error(`Owned care server did not start: ${output}`);
    };
    try {
      if (oidcProvider) provider = await (await import('./oidc-provider.mjs')).startOidcProvider();
      if (captureMail) {
        await fixtureTask('mail-up', [`MAILPIT_NAME=${mailName}`]);
        mailStarted = true;
        const smtp = await fixtureTask('mail-port', [`MAILPIT_NAME=${mailName}`, 'MAILPIT_PORT=1025/tcp']);
        const api = await fixtureTask('mail-port', [`MAILPIT_NAME=${mailName}`, 'MAILPIT_PORT=8025/tcp']);
        if (!/^127\.0\.0\.1:\d+$/.test(smtp) || !/^127\.0\.0\.1:\d+$/.test(api)) throw Error('Mailpit must publish only owned loopback ports');
        smtpPort = smtp.split(':')[1];
        mailpitUrl = `http://${api}`;
        const deadline = Date.now() + 10000;
        let ready = false;
        while (Date.now() < deadline) {
          try { ready = (await fetch(`${mailpitUrl}/readyz`, { signal: AbortSignal.timeout(1000) })).ok; } catch {}
          if (ready) break;
          await new Promise(resolve => setTimeout(resolve, 100));
        }
        if (!ready) throw Error('Owned Mailpit did not become ready');
      }
      if (!captureMail && registrationInviteOnly === undefined) await measure('snapshot', () => fixtureTask('snapshot', [`SNAPSHOT_PATH=${snapshotPath}`]));
      await measure('application-start', start);
      await use({ origin, mailpitUrl, runtimeTimings, provider,
        reset: () => measure('reset', () => fixtureTask('reset', [`SNAPSHOT_PATH=${snapshotPath}`])),
        probe: async () => JSON.parse(await fixtureTask('probe')), seedBarcodeMetadata: () => fixtureTask('seed-barcode-metadata'), barcodeMetadataProbe: async () => JSON.parse(await fixtureTask('barcode-metadata-probe')), seedLegacyCareHours: () => fixtureTask('legacy-care-hours'), legacyCareHoursProbe: async () => JSON.parse(await fixtureTask('legacy-care-hours-probe')), seedLegacySecondTaperHour: () => fixtureTask('legacy-second-taper-hour'), revoke: () => fixtureTask('revoke'), reactivate: () => fixtureTask('reactivate'),
        seedOtp: () => fixtureTask('seed-otp'), otpProbe: async () => JSON.parse((await fixtureTask('otp-probe')) || 'null'), closeOtpAccount: () => fixtureTask('close-otp-account'),
        seedRecovery: () => fixtureTask('seed-recovery'), recoveryProbe: async () => JSON.parse(await fixtureTask('recovery-probe')), exhaustOtp: () => fixtureTask('exhaust-otp'),
        seedRecoveryPasskey: () => fixtureTask('seed-recovery-passkey'),
        peopleProbe: async () => JSON.parse(await fixtureTask('people-probe')),
        restockProbe: async () => JSON.parse(await fixtureTask('restock-probe')), failRestockAudit: () => fixtureTask('fail-restock-audit'),
        seedAdministration: () => fixtureTask('seed-administration'), administrationProbe: async () => JSON.parse(await fixtureTask('administration-probe')), personViewOnly: () => fixtureTask('person-view-only'), revokePersonViewOnly: () => fixtureTask('revoke-person-view-only'), personZeroHours: () => fixtureTask('person-zero-hours'), mixedRecordability: () => fixtureTask('mixed-recordability'),
        seedPlatform: () => fixtureTask('seed-platform'), platformProbe: async () => JSON.parse(await fixtureTask('platform-probe')),
        seedSupportHistory: () => fixtureTask('seed-support-history'),
        invitationProbe: async email => JSON.parse(await fixtureTask('invitation-probe', [`INVITATION_EMAIL=${email}`])), expireInvitation: email => fixtureTask('expire-invitation', [`INVITATION_EMAIL=${email}`]),
        seedInvitationAccount: () => fixtureTask('seed-invitation-account'), seedInvitationOtp: () => fixtureTask('seed-invitation-otp'),
        invitationAcceptanceProbe: async () => JSON.parse(await fixtureTask('invitation-acceptance-probe')),
        invitationSignupProbe: async email => JSON.parse(await fixtureTask('invitation-signup-probe', [`INVITATION_EMAIL=${email}`])),
        signupDiagnostics: () => output.split('\n').filter(line => line.includes('Account setup operation failed')).join('\n'),
        logsContain: value => output.includes(value),
        frameworkSessionPeer: () => fixtureTask('framework-session-peer'),
        ageFrameworkIdle: () => fixtureTask('age-framework-idle'), ageFrameworkAbsolute: () => fixtureTask('age-framework-absolute'), expireFrameworkIdle: () => fixtureTask('expire-framework-idle'),
        agePendingFactorSession: () => fixtureTask('age-pending-factor-session'), frameworkSessionLimits: async () => JSON.parse(await fixtureTask('framework-session-limits')),
        expireAuthenticationLimits: () => fixtureTask('expire-authentication-limits'),
        failureDiagnostics: () => output.split('\n').filter(line => /Account (confirmation|setup operation) (failed|rejected)/.test(line)).slice(-20).map(line => line.replace(/https?:\/\/\S+/g, '[redacted URL]').replace(/\b(token|password|authorization|cookie|secret)\s*[=:]\s*[^\s,]+/gi, '$1=[redacted]')).join('\n').slice(-4096),
        revokeInvitation: email => fixtureTask('revoke-invitation', [`INVITATION_EMAIL=${email}`]),
        setRegistrationPolicy: inviteOnly => fixtureTask('registration-policy', [`REGISTRATION_INVITE_ONLY=${Boolean(inviteOnly)}`]), clearRegistrationPolicy: () => fixtureTask('clear-registration-policy'),
        registrationProbe: async email => JSON.parse(await fixtureTask('registration-probe', [`REGISTRATION_EMAIL=${email}`])),
        verificationProbe: async email => JSON.parse(await fixtureTask('verification-probe', [`REGISTRATION_EMAIL=${email}`])),
        ageVerificationEmail: email => fixtureTask('age-verification-email', [`REGISTRATION_EMAIL=${email}`]),
        orderProbe: async () => JSON.parse(await fixtureTask('order-probe')), failOrderAudit: () => fixtureTask('fail-order-audit'), useOrderMember: () => fixtureTask('use-order-member'),
        occurrenceProbe: async () => JSON.parse(await fixtureTask('occurrence-probe')), failOccurrenceAudit: () => fixtureTask('fail-occurrence-audit'),
        treatmentProbe: async () => JSON.parse(await fixtureTask('treatment-probe')), failTreatmentAudit: () => fixtureTask('fail-treatment-audit'),
        passkeyProbe: async () => JSON.parse(await fixtureTask('passkey-probe')),
        passkeyCounterAhead: () => fixtureTask('passkey-counter-ahead'),
        passkeyRemovalRace: () => fixtureTask('passkey-removal-race'), passkeyRemovalRaceReady: async () => (await fixtureTask('passkey-removal-race-ready')) === 't',
        seedForeignPasskey: () => fixtureTask('seed-foreign-passkey'),
        seedUnsupportedPasskey: publicKey => {
          if (typeof publicKey !== 'string' || !/^[A-Za-z0-9_-]+$/.test(publicKey)) throw Error('Invalid unsupported public passkey fixture');
          return fixtureTask('seed-unsupported-passkey', [`UNSUPPORTED_PUBLIC_KEY=${publicKey}`]);
        },
        seedRetainedPasskey: stored => {
          const fields = ['webauthn_id', 'public_key', 'user_handle'];
          if (fields.some(field => typeof stored[field] !== 'string' || !/^[A-Za-z0-9_-]+$/.test(stored[field])) || !/^\d+$/.test(String(stored.sign_count))) throw Error('Invalid retained public passkey fixture');
          return fixtureTask('seed-retained-passkey', [...fields.map(field => `RETAINED_${field.toUpperCase()}=${stored[field]}`), `RETAINED_SIGN_COUNT=${stored.sign_count}`]);
        },
        secondPerson: () => fixtureTask('second-person'),
        scheduledMedicine: () => fixtureTask('scheduled-medicine'), scheduleOnlyMedicine: () => fixtureTask('schedule-only-medicine'), dateSensitiveSchedule: () => fixtureTask('date-sensitive-schedule'), offdaySchedule: () => fixtureTask('offday-schedule'), changeTaperStep: () => fixtureTask('change-taper-step'), londonPreviewZone: () => fixtureTask('london-preview-zone'), collidingDoseSources: () => fixtureTask('colliding-dose-sources'), emptyStandardDosage: () => fixtureTask('empty-standard-dosage'),
        seedReport: () => fixtureTask('report-seed'),
        seedReportHistory: () => fixtureTask('report-history-seed'),
        reportAuditProbe: async () => JSON.parse(await fixtureTask('report-audit-probe')),
        seedReviewReport: () => fixtureTask('review-report-seed'),
        seedRefreshTrap: () => fixtureTask('report-refresh-trap'),
        revokeGrantOnRefresh: () => fixtureTask('report-revoke-grant-on-refresh'),
        revokeSessionOnRefresh: () => fixtureTask('report-revoke-session-on-refresh'),
        oauthProbe: async () => JSON.parse(await fixtureTask('oauth-probe')), ageAuthentication: () => fixtureTask('age-authentication'),
        failOAuthAudit: () => fixtureTask('fail-oauth-audit'), restoreOAuthAudit: () => fixtureTask('restore-oauth-audit'),
        failRecoveryAudit: () => fixtureTask('fail-recovery-audit'), restoreRecoveryAudit: () => fixtureTask('restore-recovery-audit'),
        failPersonalKeyAudit: () => fixtureTask('fail-personal-key-audit'), restorePersonalKeyAudit: () => fixtureTask('restore-personal-key-audit'), expirePersonalKeys: () => fixtureTask('expire-personal-keys'), withdrawKeyMembership: () => fixtureTask('withdraw-key-membership'),
        failProviderAudit: () => fixtureTask('fail-provider-audit'), restoreProviderAudit: () => fixtureTask('restore-provider-audit'), providerLinkProbe: async () => JSON.parse(await fixtureTask('provider-link-probe')),
        failTotpAudit: () => fixtureTask('fail-totp-audit'), restoreTotpAudit: () => fixtureTask('restore-totp-audit'),
        failEmailAudit: () => fixtureTask('fail-email-audit'), restoreEmailAudit: () => fixtureTask('restore-email-audit'),
        emailChangeProbe: async () => JSON.parse(await fixtureTask('email-change-probe')),
        transferClosureOwnership: () => fixtureTask('transfer-closure-ownership'), closureProbe: async () => JSON.parse(await fixtureTask('closure-probe')),
        failClosureAudit: () => fixtureTask('fail-closure-audit'), restoreClosureAudit: () => fixtureTask('restore-closure-audit'),
        expireTotpLock: () => fixtureTask('expire-totp-lock'),
        clearAdoptedOnboarding: () => fixtureTask('clear-adopted-onboarding'),
        ageRetainedFactorUse: () => fixtureTask('age-retained-factor-use'),
        expireRetainedFactorLock: () => fixtureTask('expire-retained-factor-lock'), failRetainedFactorAudit: () => fixtureTask('fail-retained-factor-audit'),
        securityRevocationRace: () => fixtureTask('security-revocation-race'), securityRevocationRaceReady: async () => (await fixtureTask('security-revocation-race-ready')) === 't',
        securityFactorRace: () => fixtureTask('security-factor-race'), securityFactorRaceReady: async () => (await fixtureTask('security-factor-race-ready')) === 't',
        restart: async () => { await measure('application-stop', () => stopOwnedProcess(server)); await measure('application-start', start); },
        revokeSession: () => fixtureTask('revoke-session'), expireSession: () => fixtureTask('expire-session'), doseRequestId: () => fixtureTask('dose-request-id') });
    } finally {
      if (reservation.listening) await new Promise(resolve => reservation.close(resolve));
      try { if (server) await measure('application-stop', () => stopOwnedProcess(server)); } finally {
        try { if (mailStarted) await fixtureTask('mail-down', [`MAILPIT_NAME=${mailName}`]); } finally { try { if (provider) await provider.close(); } finally { await rm(snapshotDirectory, { recursive: true, force: true }); } }
      }
    }
  }); } finally {
    if (databaseOwnership) {
      const teardown = databaseOwnership.timings.find(phase => phase.name === 'foundation:db-down');
      if (teardown) runtimeTimings.phases.push({ name: 'database-stop', durationMs: teardown.durationMs });
    }
    await mkdir(join(root, 'test-results'), { recursive: true });
    await writeFile(join(root, 'test-results', `runtime-${randomUUID()}.json`), JSON.stringify({
      captureMail, registrationInviteOnly, totalRuntimeMs: performance.now() - databaseStarted, ...runtimeTimings
    }), { mode: 0o600 });
  }
}

export const test = base.extend({
  captureMail: [false, { option: true }],
  registrationInviteOnly: [undefined, { option: true }],
  reportAssetsUnavailable: [false, { option: true }],
  oidcProvider: [false, { option: true }],
  isolatedCare: [false, { option: true }],
  sharedCare: [async ({}, use) => {
    let ready;
    let release;
    let completed;
    const released = new Promise(resolve => { release = resolve; });
    const pool = {
      get: () => {
        if (!ready) {
          ready = new Promise((resolve, reject) => {
            completed = withCareFixture(false, undefined, false, async fixture => { resolve(fixture); await released; });
            completed.catch(reject);
          });
        }
        return ready;
      }
    };
    try { await use(pool); } finally { release(); if (completed) await completed; }
  }, { scope: 'worker', timeout: 180000 }],
  careFixture: [async ({ captureMail, registrationInviteOnly, reportAssetsUnavailable, isolatedCare, sharedCare, oidcProvider }, use, info) => {
    const started = performance.now();
    let fixture;
    let phaseStart = 0;
    const run = async current => {
      fixture = current;
      const bodyStarted = performance.now();
      try { await use(fixture); } finally {
        fixture.runtimeTimings.phases.push({ name: 'test-and-browser-cleanup', durationMs: performance.now() - bodyStarted });
        if (info.status !== info.expectedStatus) await info.attach('application-failure-diagnostics', { body: fixture.failureDiagnostics(), contentType: 'text/plain' });
      }
    };
    try {
      if (captureMail || registrationInviteOnly !== undefined || reportAssetsUnavailable || isolatedCare || oidcProvider || info.tags.includes('@isolated-runtime')) {
        await withCareFixture(captureMail, registrationInviteOnly, reportAssetsUnavailable, run, oidcProvider);
      } else {
        fixture = await sharedCare.get();
        phaseStart = fixture.runtimeTimings.reportedPhaseCount ?? 0;
        await fixture.reset();
        await run(fixture);
      }
    } finally {
      if (fixture) { await info.attach('runtime-timings', {
        body: JSON.stringify({ totalFixtureMs: performance.now() - started, phases: fixture.runtimeTimings.phases.slice(phaseStart) }), contentType: 'application/json'
      }); fixture.runtimeTimings.reportedPhaseCount = fixture.runtimeTimings.phases.length; }
    }
  }, { timeout: 180000 }],
  baseURL: async ({ careFixture }, use) => use(careFixture.origin)
});
export { expect };
