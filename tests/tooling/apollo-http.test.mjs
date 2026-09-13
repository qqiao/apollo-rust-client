import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { requestJson } from '../../scripts/apollo-fixtures.mjs';

function createTestServer(handler) {
  const sockets = new Set();
  const server = http.createServer((req, res) => {
    handler(req, res);
  });

  server.on('connection', (socket) => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
  });

  return new Promise((resolve, reject) => {
    server.listen(0, '127.0.0.1', () => {
      const port = server.address().port;
      const baseUrl = `http://127.0.0.1:${port}`;
      resolve({
        baseUrl,
        server,
        sockets,
        close: () =>
          new Promise((done) => {
            for (const sock of sockets) {
              sock.destroy();
            }
            server.close(() => done());
          }),
      });
    });
    server.on('error', reject);
  });
}

// -----------------------------------------------------------------------------
// T01: Request deadline on withheld headers & stalled body
// -----------------------------------------------------------------------------

test('requestJson rejects on server withholding headers within configured timeout (AC-001)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    // Deliberately withhold headers: do nothing
  });

  try {
    const start = Date.now();
    await assert.rejects(
      async () => {
        await requestJson(`${fixtureServer.baseUrl}/hang-headers`, {
          timeoutMs: 150,
        });
      },
      (err) => {
        assert.match(err.message, /timed out after 150ms/i);
        assert.match(err.message, /GET http:\/\/127\.0\.0\.1:/);
        return true;
      }
    );
    const elapsed = Date.now() - start;
    assert.ok(elapsed < 2000, `Expected timeout around 150ms, took ${elapsed}ms`);
  } finally {
    await fixtureServer.close();
  }
});

test('requestJson rejects when headers arrive but body stalls beyond timeout (AC-002)', async () => {
  let socketClosed = false;
  const fixtureServer = await createTestServer((req, res) => {
    res.writeHead(200, {
      'Content-Type': 'application/json',
      'Transfer-Encoding': 'chunked',
    });
    if (typeof res.flushHeaders === 'function') {
      res.flushHeaders();
    }
    res.write('{"partial":');
    req.socket.on('close', () => {
      socketClosed = true;
    });
    // Never call res.end()
  });

  try {
    const start = Date.now();
    await assert.rejects(
      async () => {
        await requestJson(`${fixtureServer.baseUrl}/hang-body`, {
          timeoutMs: 150,
        });
      },
      (err) => {
        assert.match(err.message, /timed out after 150ms/i);
        assert.match(err.message, /GET http:\/\/127\.0\.0\.1:/);
        return true;
      }
    );
    const elapsed = Date.now() - start;
    assert.ok(elapsed < 2000, `Expected timeout around 150ms, took ${elapsed}ms`);
    // Wait a brief moment for socket close to propagate
    await new Promise((r) => setTimeout(r, 50));
    assert.ok(socketClosed, 'Underlying socket must be closed/aborted when timeout fires');
  } finally {
    await fixtureServer.close();
  }
});

// -----------------------------------------------------------------------------
// T02: Response shape compatibility, diagnostics, transport failures, timer cleanup
// -----------------------------------------------------------------------------

test('requestJson returns expected response shape for successful JSON (AC-003)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ greeting: 'hello', count: 42 }));
  });

  try {
    const res = await requestJson(`${fixtureServer.baseUrl}/data`, { timeoutMs: 500 });
    assert.equal(res.status, 200);
    assert.equal(res.ok, true);
    assert.deepEqual(res.data, { greeting: 'hello', count: 42 });
    assert.ok(res.headers.get('content-type').includes('application/json'));
  } finally {
    await fixtureServer.close();
  }
});

test('requestJson handles empty body, malformed JSON, and non-2xx status without throwing (AC-004)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    if (req.url === '/empty') {
      res.writeHead(204);
      res.end();
    } else if (req.url === '/malformed') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end('invalid { json');
    } else if (req.url === '/unauthorized') {
      res.writeHead(401, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ message: 'unauthorized' }));
    } else if (req.url === '/server-error') {
      res.writeHead(500);
      res.end('internal error text');
    }
  });

  try {
    // Empty body -> data: null
    const emptyRes = await requestJson(`${fixtureServer.baseUrl}/empty`, { timeoutMs: 500 });
    assert.equal(emptyRes.status, 204);
    assert.equal(emptyRes.ok, true);
    assert.equal(emptyRes.data, null);

    // Malformed JSON -> returns raw text
    const malformedRes = await requestJson(`${fixtureServer.baseUrl}/malformed`, { timeoutMs: 500 });
    assert.equal(malformedRes.status, 200);
    assert.equal(malformedRes.ok, true);
    assert.equal(malformedRes.data, 'invalid { json');

    // HTTP 401 -> ok: false, does not throw
    const unauthRes = await requestJson(`${fixtureServer.baseUrl}/unauthorized`, { timeoutMs: 500 });
    assert.equal(unauthRes.status, 401);
    assert.equal(unauthRes.ok, false);
    assert.deepEqual(unauthRes.data, { message: 'unauthorized' });

    // HTTP 500 -> ok: false, data is text
    const errRes = await requestJson(`${fixtureServer.baseUrl}/server-error`, { timeoutMs: 500 });
    assert.equal(errRes.status, 500);
    assert.equal(errRes.ok, false);
    assert.equal(errRes.data, 'internal error text');
  } finally {
    await fixtureServer.close();
  }
});

test('timeout error diagnostic omits Authorization headers and payload bodies (AC-005)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    // Stalls
  });

  try {
    const sensitiveSecret = 'SUPER_SECRET_KEY_12345';
    const sensitivePayload = { secretData: 'DO_NOT_LEAK' };

    await assert.rejects(
      async () => {
        await requestJson(`${fixtureServer.baseUrl}/secret-endpoint`, {
          method: 'POST',
          headers: {
            Authorization: `Apollo app:${sensitiveSecret}`,
          },
          body: sensitivePayload,
          timeoutMs: 100,
        });
      },
      (err) => {
        assert.ok(!err.message.includes(sensitiveSecret), 'Diagnostic must NOT leak secret header');
        assert.ok(!err.message.includes('DO_NOT_LEAK'), 'Diagnostic must NOT leak payload body');
        assert.match(err.message, /POST/);
        assert.match(err.message, /timed out after 100ms/);
        return true;
      }
    );
  } finally {
    await fixtureServer.close();
  }
});

test('transport failures before deadline remain transport errors and do not report timeout (AC-006)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    // Immediately destroy the connection
    req.socket.destroy();
  });

  try {
    await assert.rejects(
      async () => {
        await requestJson(`${fixtureServer.baseUrl}/immediate-reset`, {
          timeoutMs: 2000,
        });
      },
      (err) => {
        assert.doesNotMatch(err.message, /timed out/i);
        return true;
      }
    );
  } finally {
    await fixtureServer.close();
  }
});

test('timer resources are cleared on completed requests and do not abort later (FR-003)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'fast' }));
  });

  try {
    // Request with short 100ms timeout
    const res = await requestJson(`${fixtureServer.baseUrl}/quick`, { timeoutMs: 100 });
    assert.equal(res.status, 200);

    // Wait past the timeout to verify no uncaught exception or lingering timer fires
    await new Promise((r) => setTimeout(r, 150));
  } finally {
    await fixtureServer.close();
  }
});
