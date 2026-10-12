import { createServer, request } from 'node:http';

export async function workerUpgradeServer(origin, baseline) {
  let worker = baseline;
  let workerRequests = 0;
  const upstream = new URL(origin);
  const server = createServer((incoming, outgoing) => {
    if (incoming.url === '/profile-service-worker.js') {
      workerRequests += 1;
      outgoing.writeHead(200, { 'Content-Type': 'application/javascript', 'Service-Worker-Allowed': '/', 'Cache-Control': 'no-store' });
      outgoing.end(worker);
      return;
    }
    const headers = { ...incoming.headers, host: upstream.host };
    if (headers.origin === `http://${incoming.headers.host}`) headers.origin = upstream.origin;
    if (headers.referer?.startsWith(`http://${incoming.headers.host}/`)) headers.referer = upstream.origin + new URL(headers.referer).pathname;
    const proxy = request({ hostname: '127.0.0.1', port: upstream.port, method: incoming.method, path: incoming.url, headers }, response => {
      outgoing.writeHead(response.statusCode, response.headers);
      response.pipe(outgoing);
    });
    proxy.on('error', () => { outgoing.writeHead(502); outgoing.end(); });
    incoming.pipe(proxy);
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  return {
    origin: `http://localhost:${server.address().port}`,
    setWorker: source => { worker = source; },
    get workerRequests() { return workerRequests; },
    close: async () => {
      server.closeAllConnections();
      await new Promise(resolve => server.close(resolve));
    },
  };
}
