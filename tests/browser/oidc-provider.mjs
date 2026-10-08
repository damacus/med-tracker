import { createServer } from 'node:http';
import { randomUUID, createHash } from 'node:crypto';
import { generateKeyPair, exportJWK, SignJWT } from 'jose';

export async function startOidcProvider() {
  const keys = await generateKeyPair('RS256');
  const foreign = await generateKeyPair('RS256');
  const jwk = { ...await exportJWK(keys.publicKey), kid: 'owned-fixture-key', alg: 'RS256', use: 'sig' };
  const codes = new Map();
  let origin;
  let mode = 'valid';
  let claims = { sub: 'synthetic-zitadel-subject', email: 'persistence@example.test', email_verified: true, name: 'Synthetic provider account' };
  const observations = { pkce: false, nonce: false, codeFlow: false, freshRequested: false };
  const json = (response, status, body) => { response.writeHead(status, { 'content-type': 'application/json', 'cache-control': 'no-store' }); response.end(JSON.stringify(body)); };
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url, origin);
      if (mode === 'outage') return json(response, 503, { error: 'temporarily_unavailable' });
      if (url.pathname === '/.well-known/openid-configuration') return json(response, 200, {
        issuer: origin, authorization_endpoint: `${origin}/authorize`, token_endpoint: `${origin}/token`, jwks_uri: `${origin}/jwks`,
        response_types_supported: ['code'], subject_types_supported: ['public'], id_token_signing_alg_values_supported: ['RS256'],
        token_endpoint_auth_methods_supported: ['client_secret_basic'], scopes_supported: ['openid', 'email', 'profile'], code_challenge_methods_supported: ['S256']
      });
      if (url.pathname === '/jwks') return json(response, 200, { keys: [jwk] });
      if (url.pathname === '/authorize') {
        observations.nonce = Boolean(url.searchParams.get('nonce'));
        observations.codeFlow = url.searchParams.get('response_type') === 'code';
        observations.freshRequested = url.searchParams.get('max_age') === '0';
        const callback = new URL(url.searchParams.get('redirect_uri'));
        if (callback.hostname !== 'localhost') return json(response, 400, { error: 'invalid_request' });
        const code = randomUUID();
        codes.set(code, { nonce: url.searchParams.get('nonce'), challenge: url.searchParams.get('code_challenge'), method: url.searchParams.get('code_challenge_method'), redirect: callback.href, mode, claims: { ...claims } });
        callback.searchParams.set('code', code);
        callback.searchParams.set('state', mode === 'bad_state' ? randomUUID() : url.searchParams.get('state'));
        response.writeHead(302, { location: callback.href, 'cache-control': 'no-store' });
        return response.end();
      }
      if (url.pathname === '/token' && request.method === 'POST') {
        const chunks = [];
        for await (const chunk of request) chunks.push(chunk);
        const form = new URLSearchParams(Buffer.concat(chunks).toString());
        const grant = codes.get(form.get('code'));
        codes.delete(form.get('code'));
        if (!grant || request.headers.authorization !== `Basic ${Buffer.from('medtracker-fixture:password').toString('base64')}`) return json(response, 400, { error: 'invalid_grant' });
        const challenge = createHash('sha256').update(form.get('code_verifier') || '').digest('base64url');
        observations.pkce = grant.method === 'S256' && challenge === grant.challenge && form.get('redirect_uri') === grant.redirect;
        if (!observations.pkce || grant.mode === 'reject_pkce') return json(response, 400, { error: 'invalid_grant' });
        const now = Math.floor(Date.now() / 1000);
        const payload = { ...grant.claims, nonce: grant.mode === 'bad_nonce' ? randomUUID() : grant.nonce };
        if (grant.mode !== 'missing_auth_time') payload.auth_time = now + (grant.mode === 'future_auth_time' ? 3600 : grant.mode === 'stale_auth_time' ? -600 : 0);
        const token = await new SignJWT(payload).setProtectedHeader({ alg: 'RS256', kid: jwk.kid }).setIssuer(grant.mode === 'bad_issuer' ? `${origin}/other` : origin).setAudience(grant.mode === 'bad_audience' ? 'other-client' : 'medtracker-fixture').setIssuedAt(now).setExpirationTime(grant.mode === 'expired' ? now - 1 : now + 300).sign(grant.mode === 'bad_signature' ? foreign.privateKey : keys.privateKey);
        return json(response, 200, { access_token: randomUUID(), token_type: 'Bearer', expires_in: 300, id_token: token });
      }
      return json(response, 404, { error: 'not_found' });
    } catch { return json(response, 500, { error: 'fixture_failure' }); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
  return { origin, observations, setMode: value => { mode = value; }, setClaims: value => { claims = { ...claims, ...value }; }, close: () => new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve())) };
}
