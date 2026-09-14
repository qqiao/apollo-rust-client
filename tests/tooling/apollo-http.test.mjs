import test from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { requestJson, redactUrl } from '../../scripts/apollo-fixtures.mjs';

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

test('redactUrl sanitizes basic auth credentials and sensitive query parameters (AC-001, AC-002, AC-003)', () => {
  // Test basic auth userinfo redaction
  const urlWithAuth = 'http://admin:super_secret_pw@127.0.0.1:8080/configs?env=DEV';
  const redactedAuth = redactUrl(urlWithAuth);
  assert.ok(!redactedAuth.includes('super_secret_pw'), 'Must not contain password');
  assert.match(redactedAuth, /\*\*\*:\*\*\*@127\.0\.0\.1:8080\/configs/);
  assert.match(redactedAuth, /env=DEV/);

  // Test sensitive query parameters redaction
  const urlWithTokens = 'http://127.0.0.1:8080/api?token=secret123&client_secret=topsecret&key=mykey&password=pw&auth=bearer&normal_param=preserve_me';
  const redactedTokens = redactUrl(urlWithTokens);
  assert.ok(!redactedTokens.includes('secret123'), 'Must not contain token value');
  assert.ok(!redactedTokens.includes('topsecret'), 'Must not contain client_secret value');
  assert.ok(!redactedTokens.includes('mykey'), 'Must not contain key value');
  assert.match(redactedTokens, /token=REDACTED/);
  assert.match(redactedTokens, /client_secret=REDACTED/);
  assert.match(redactedTokens, /normal_param=preserve_me/);

  // Test standard URL without secrets is preserved
  const plainUrl = 'http://127.0.0.1:8080/apps/sample-app?format=json';
  assert.equal(redactUrl(plainUrl), plainUrl);
});

test('timeout error diagnostic redacts credentials and query secrets from URL (AC-001, AC-002)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    // Stalls indefinitely
  });

  try {
    const rawUrl = `${fixtureServer.baseUrl.replace('http://', 'http://user:secret_password@')}/sensitive-path?token=token_secret_xyz&appId=testApp`;

    await assert.rejects(
      async () => {
        await requestJson(rawUrl, {
          timeoutMs: 100,
        });
      },
      (err) => {
        assert.ok(!err.message.includes('secret_password'), 'Diagnostic message must NOT leak basic auth password');
        assert.ok(!err.message.includes('token_secret_xyz'), 'Diagnostic message must NOT leak token query param');
        assert.match(err.message, /sensitive-path/);
        assert.match(err.message, /appId=testApp/);
        assert.match(err.message, /token=REDACTED/);
        return true;
      }
    );
  } finally {
    await fixtureServer.close();
  }
});

test('redactUrl handles URL object input and requestJson redacts URL object timeouts (P1)', async () => {
  // Test redactUrl with URL object
  const urlObj = new URL('http://admin:super_secret_pw@127.0.0.1:8080/configs?token=secret123&env=DEV');
  const redacted = redactUrl(urlObj);
  assert.equal(typeof redacted, 'string');
  assert.ok(!redacted.includes('super_secret_pw'), 'Must not contain password');
  assert.ok(!redacted.includes('secret123'), 'Must not contain token');
  assert.match(redacted, /\*\*\*:\*\*\*@127\.0\.0\.1:8080\/configs/);
  assert.match(redacted, /token=REDACTED/);
  assert.match(redacted, /env=DEV/);

  // Test requestJson with URL object timeout
  const fixtureServer = await createTestServer((req, res) => {
    // Stalls
  });

  try {
    const timeoutUrlObj = new URL(`${fixtureServer.baseUrl}/stall?token=SYNTHETIC_SECRET&client_secret=TOP_SECRET`);
    await assert.rejects(
      async () => {
        await requestJson(timeoutUrlObj, { timeoutMs: 100 });
      },
      (err) => {
        assert.ok(!err.message.includes('SYNTHETIC_SECRET'), 'Timeout message must NOT contain SYNTHETIC_SECRET');
        assert.ok(!err.message.includes('TOP_SECRET'), 'Timeout message must NOT contain TOP_SECRET');
        assert.match(err.message, /token=REDACTED/);
        assert.match(err.message, /client_secret=REDACTED/);
        return true;
      }
    );
  } finally {
    await fixtureServer.close();
  }
});

test('requestJson correctly decodes percent-encoded basic auth credentials and respects header precedence (P2)', async () => {
  const authServer = await createTestServer((req, res) => {
    if (req.url.startsWith('/auth')) {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ authorization: req.headers.authorization }));
      return;
    }
    res.writeHead(404);
    res.end();
  });

  try {
    // Percent-encoded password p%40ss must decode to p@ss
    const authUrl = `${authServer.baseUrl.replace('http://', 'http://user%3Atest:p%40ss@')}/auth`;
    const res = await requestJson(authUrl);
    assert.equal(res.status, 200);
    assert.ok(res.data.authorization.startsWith('Basic '));
    const decodedCreds = Buffer.from(res.data.authorization.slice(6), 'base64').toString('utf8');
    assert.equal(decodedCreds, 'user:test:p@ss', 'Percent-encoded user:test and p%40ss must decode properly');

    // Explicit caller Authorization header overrides URL credentials
    const customAuth = await requestJson(authUrl, {
      headers: { 'Authorization': 'Bearer custom_override_token' },
    });
    assert.equal(customAuth.data.authorization, 'Bearer custom_override_token');

    // Case-insensitive AUTHORIZATION header overrides URL credentials
    const uppercaseAuth = await requestJson(authUrl, {
      headers: { 'AUTHORIZATION': 'Bearer uppercase_override' },
    });
    assert.equal(uppercaseAuth.data.authorization, 'Bearer uppercase_override');

    // Standard Headers instance overrides URL credentials and is preserved
    const headersInstance = new Headers();
    headersInstance.set('authorization', 'Bearer headers_instance_token');
    headersInstance.set('X-Custom-Header', 'custom_val');
    const instanceAuth = await requestJson(authUrl, {
      headers: headersInstance,
    });
    assert.equal(instanceAuth.data.authorization, 'Bearer headers_instance_token');
  } finally {
    await authServer.close();
  }
});

test('redactUrl covers common secret query forms, duplicate keys, and preserves ordinary parameters (R3-2 / 014/AC-003)', async () => {
  // 1. Table of sensitive keys in various naming forms (camelCase, snake_case, kebab-case, UPPERCASE)
  const sensitiveCases = [
    ['appSecret', 'appSecretVal'],
    ['app_secret', 'app_secret_val'],
    ['app-secret', 'app-secret-val'],
    ['APP_SECRET', 'APP_SECRET_VAL'],
    ['clientSecret', 'clientSecretVal'],
    ['client_secret', 'client_secret_val'],
    ['client-secret', 'client-secret-val'],
    ['CLIENT_SECRET', 'CLIENT_SECRET_VAL'],
    ['apiKey', 'apiKeyVal'],
    ['api_key', 'api_key_val'],
    ['api-key', 'api-key-val'],
    ['API_KEY', 'API_KEY_VAL'],
    ['accessKey', 'accessKeyVal'],
    ['access_key', 'access_key_val'],
    ['access-key', 'access-key-val'],
    ['ACCESS_KEY', 'ACCESS_KEY_VAL'],
    ['accessToken', 'accessTokenVal'],
    ['access_token', 'access_token_val'],
    ['access-token', 'access-token-val'],
    ['authorization', 'authHeaderVal'],
    ['AUTHORIZATION', 'AUTHHEADERVAL'],
    ['token', 'tokVal'],
    ['secret', 'secVal'],
    ['key', 'keyVal'],
    ['password', 'pwVal'],
    ['auth', 'authVal'],
    ['credential', 'credVal'],
    ['credentials', 'credsVal'],
  ];

  for (const [key, val] of sensitiveCases) {
    const raw = `http://127.0.0.1:8080/configs?${key}=${val}&env=DEV&releaseKey=20260914`;
    const redacted = redactUrl(raw);
    assert.ok(!redacted.includes(val), `Key '${key}' with value '${val}' must be redacted`);
    assert.match(redacted, new RegExp(`${key}=REDACTED`, 'i'), `Must show ${key}=REDACTED`);
    assert.match(redacted, /env=DEV/, `Ordinary param env=DEV must be preserved for ${key}`);
    assert.match(redacted, /releaseKey=20260914/, `releaseKey must be preserved as ordinary param for ${key}`);
  }

  // 2. Duplicate sensitive query keys must all be redacted without leaking values
  const dupUrl = 'http://127.0.0.1:8080/configs?apiKey=secret_one&apiKey=secret_two&appSecret=app_sec1&appSecret=app_sec2&appId=SampleApp';
  const redactedDup = redactUrl(dupUrl);
  assert.ok(!redactedDup.includes('secret_one'), 'First duplicate key value must not leak');
  assert.ok(!redactedDup.includes('secret_two'), 'Second duplicate key value must not leak');
  assert.ok(!redactedDup.includes('app_sec1'), 'First duplicate appSecret must not leak');
  assert.ok(!redactedDup.includes('app_sec2'), 'Second duplicate appSecret must not leak');
  assert.match(redactedDup, /appId=SampleApp/, 'Ordinary appId must be preserved');

  // 3. Fallback relative and malformed URLs
  const relativeUrl = '/api/v1/configs?appSecret=rel_secret&apiKey=rel_key&cluster=default';
  const redactedRel = redactUrl(relativeUrl);
  assert.ok(!redactedRel.includes('rel_secret'), 'Relative URL secret must be redacted');
  assert.ok(!redactedRel.includes('rel_key'), 'Relative URL apiKey must be redacted');
  assert.match(redactedRel, /cluster=default/, 'Ordinary cluster parameter in relative URL must be preserved');

  // 4. Real request timeout diagnostic redaction with extended secret forms and duplicate keys
  const fixtureServer = await createTestServer((req, res) => {
    // Hangs
  });
  try {
    const timeoutUrl = `${fixtureServer.baseUrl}/hang?appSecret=SYNTH_APP_SEC&apiKey=SYNTH_API_KEY1&apiKey=SYNTH_API_KEY2&accessKey=SYNTH_ACC_KEY&authorization=SYNTH_AUTH&cluster=testCluster`;
    await assert.rejects(
      async () => {
        await requestJson(timeoutUrl, { timeoutMs: 100 });
      },
      (err) => {
        assert.ok(!err.message.includes('SYNTH_APP_SEC'), 'Must not leak SYNTH_APP_SEC');
        assert.ok(!err.message.includes('SYNTH_API_KEY1'), 'Must not leak SYNTH_API_KEY1');
        assert.ok(!err.message.includes('SYNTH_API_KEY2'), 'Must not leak SYNTH_API_KEY2');
        assert.ok(!err.message.includes('SYNTH_ACC_KEY'), 'Must not leak SYNTH_ACC_KEY');
        assert.ok(!err.message.includes('SYNTH_AUTH'), 'Must not leak SYNTH_AUTH');
        assert.match(err.message, /cluster=testCluster/, 'Must preserve cluster in timeout error');
        return true;
      }
    );
  } finally {
    await fixtureServer.close();
  }
});

test('requestJson respects caller-supplied fetchOptions.signal and cancels before deadline (C1)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    // Hangs
  });

  try {
    const callerController = new AbortController();
    const abortPromise = (async () => {
      await new Promise((r) => setTimeout(r, 50));
      callerController.abort(new Error('Caller operation cancelled'));
    })();

    const start = Date.now();
    await assert.rejects(
      async () => {
        await requestJson(`${fixtureServer.baseUrl}/hang`, {
          timeoutMs: 5000,
          signal: callerController.signal,
        });
      },
      (err) => {
        assert.notEqual(err.name, 'TimeoutError', 'Must not be classified as TimeoutError');
        assert.match(err.message, /Caller operation cancelled|aborted/i);
        return true;
      }
    );
    const elapsed = Date.now() - start;
    assert.ok(elapsed < 2000, `Expected early cancel around 50ms, took ${elapsed}ms`);
    await abortPromise;
  } finally {
    await fixtureServer.close();
  }
});

test('requestJson rejects immediately when caller-supplied signal is already aborted (C1)', async () => {
  const callerController = new AbortController();
  callerController.abort(new Error('Already aborted'));

  await assert.rejects(
    async () => {
      await requestJson('http://127.0.0.1:9999/dummy', {
        timeoutMs: 5000,
        signal: callerController.signal,
      });
    },
    (err) => {
      assert.notEqual(err.name, 'TimeoutError');
      assert.match(err.message, /Already aborted|aborted/i);
      return true;
    }
  );
});

test('requestJson still reports TimeoutError when caller-supplied signal is present but not aborted (C1)', async () => {
  const fixtureServer = await createTestServer((req, res) => {
    // Hangs
  });

  try {
    const callerController = new AbortController();
    await assert.rejects(
      async () => {
        await requestJson(`${fixtureServer.baseUrl}/hang`, {
          timeoutMs: 100,
          signal: callerController.signal,
        });
      },
      (err) => {
        assert.equal(err.name, 'TimeoutError', 'Must be classified as TimeoutError');
        assert.match(err.message, /timed out after 100ms/i);
        return true;
      }
    );
  } finally {
    await fixtureServer.close();
  }
});
