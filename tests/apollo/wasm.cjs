/**
 * Real Apollo integration test suite for Node.js / WebAssembly.
 *
 * Tests the consumer-facing generated JavaScript / WASM package against a
 * genuine running Apollo ConfigService instance using real Node.js networking.
 */

'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const wasmPackagePath = process.env.APOLLO_TEST_WASM_PACKAGE;
if (!wasmPackagePath) {
  throw new Error('APOLLO_TEST_WASM_PACKAGE environment variable is required');
}
const wasm = require(path.resolve(wasmPackagePath));
const { Client, ClientConfig, Properties } = wasm;

const CONFIG_URL = process.env.APOLLO_TEST_CONFIG_URL || 'http://127.0.0.1:8080';
const ADMIN_URL = process.env.APOLLO_TEST_ADMIN_URL || 'http://127.0.0.1:8090';
const RUN_ID = process.env.APOLLO_TEST_RUN_ID || 'default-run-id';
const FIXTURES_SCRIPT = path.resolve(__dirname, '../../scripts/apollo-fixtures.mjs');

function runFixtureCommand(args) {
  return execFileSync(process.execPath, [FIXTURES_SCRIPT, ...args], {
    encoding: 'utf8',
    env: process.env,
    timeout: 30000,
  });
}

async function waitForConfigServiceValue(appId, cluster, namespace, expectedKey, expectedVal, timeoutMs = 60000) {
  const url = `${CONFIG_URL.replace(/\/+$/, '')}/configfiles/json/${encodeURIComponent(appId)}/${encodeURIComponent(cluster)}/${encodeURIComponent(namespace)}`;
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(url);
      if (resp.ok) {
        const data = await resp.json();
        if (data && data[expectedKey] === expectedVal) {
          return;
        }
      }
    } catch {
      // transient network probe error; continue polling
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error(`Timed out waiting for ConfigService at ${url} to return ${expectedKey}=${expectedVal}`);
}

test('real_apollo_wasm_export_smoke', () => {
  assert.equal(typeof wasm.Client, 'function', 'Client should be exported as a constructor');
  assert.equal(typeof wasm.ClientConfig, 'function', 'ClientConfig should be exported as a constructor');
  assert.equal(typeof wasm.Properties, 'function', 'Properties should be exported as a constructor');
  assert.equal(typeof wasm.Client.prototype.namespace, 'function', 'Client.prototype.namespace should be a function');
  assert.equal(typeof wasm.Client.prototype.add_listener, 'function', 'Client.prototype.add_listener should be a function');
  assert.equal(typeof wasm.Client.prototype.start, 'function', 'Client.prototype.start should be a function');
  assert.equal(typeof wasm.Client.prototype.stop, 'function', 'Client.prototype.stop should be a function');
});

test('real_apollo_wasm_formats_and_identity', async () => {
  // App 101010101, default cluster: formats and associated namespaces
  const config1 = new ClientConfig('101010101', CONFIG_URL, 'default');
  const client1 = new Client(config1);

  try {
    // 1. Properties format
    const props = await client1.namespace('application');
    try {
      assert.equal(props.get_string('stringValue'), 'string value');
      assert.equal(props.get_int('intValue'), 42n);
      assert.equal(props.get_float('floatValue'), 4.2);
      assert.equal(props.get_bool('boolValue'), false);
      assert.equal(props.get_string('fixtureRunId'), RUN_ID);
      assert.equal(props.get_string('missingValue'), undefined);
    } finally {
      props.free();
    }

    // 2. Non-application properties namespace
    const configNs = await client1.namespace('config');
    try {
      assert.equal(configNs.get_string('publicValue'), 'properties');
    } finally {
      configNs.free();
    }

    // 2b. Wire normalization: "config.properties" alias for "config"
    const configPropNs = await client1.namespace('config.properties');
    try {
      assert.equal(configPropNs.get_string('publicValue'), 'properties');
    } finally {
      configPropNs.free();
    }

    // 3. JSON format
    const jsonNs = await client1.namespace('application.json');
    assert.deepEqual(jsonNs, { host: 'localhost', port: 8080, run: true });

    // 4. YAML format (.yml)
    const ymlNs = await client1.namespace('application.yml');
    assert.deepEqual(ymlNs, { host: 'localhost', port: 8080, run: true });

    // 5. YAML format (.yaml)
    const yamlNs = await client1.namespace('application.yaml');
    assert.deepEqual(yamlNs, { host: 'localhost', port: 8080, run: true });

    // 6. Text format (.txt)
    const textNs = await client1.namespace('readme.txt');
    assert.equal(textNs, 'plain text configuration\nsecond line\n');

    // 7. Associated public namespace
    const assocNs = await client1.namespace('FX.apollo');
    try {
      assert.equal(assocNs.get_string('publicValue'), 'associated');
      assert.equal(assocNs.get_string('identity'), 'public-owner');
    } finally {
      assocNs.free();
    }
  } finally {
    client1.free();
  }

  // App 101010101, custom cluster 'integration'
  const config2 = new ClientConfig('101010101', CONFIG_URL, 'integration');
  const client2 = new Client(config2);

  try {
    const clusterNs = await client2.namespace('application');
    try {
      assert.equal(clusterNs.get_string('stringValue'), 'cluster value');
      assert.equal(clusterNs.get_string('identity'), 'plain-integration');
      assert.equal(clusterNs.get_string('fixtureRunId'), RUN_ID);
    } finally {
      clusterNs.free();
    }
  } finally {
    client2.free();
  }
});

test('real_apollo_wasm_access_key', async () => {
  const secret = 'apollo-integration-only-secret-v1';

  // Negative 1: Missing secret on authenticated app
  const configNoSecret = new ClientConfig('101010102', CONFIG_URL, 'default');
  const clientNoSecret = new Client(configNoSecret);
  try {
    await assert.rejects(
      async () => {
        await clientNoSecret.namespace('application');
      },
      (err) => String(err).includes('401')
    );
  } finally {
    clientNoSecret.free();
  }

  // Negative 2: Incorrect secret on authenticated app
  const configBadSecret = new ClientConfig('101010102', CONFIG_URL, 'default');
  configBadSecret.secret = 'wrong-integration-secret';
  const clientBadSecret = new Client(configBadSecret);
  try {
    await assert.rejects(
      async () => {
        await clientBadSecret.namespace('application');
      },
      (err) => String(err).includes('401')
    );
  } finally {
    clientBadSecret.free();
  }

  // Positive: Valid secret on authenticated app
  const configValid = new ClientConfig('101010102', CONFIG_URL, 'default');
  configValid.secret = secret;
  const clientValid = new Client(configValid);
  try {
    const props = await clientValid.namespace('application');
    try {
      assert.equal(props.get_string('stringValue'), 'string value');
      assert.equal(props.get_string('identity'), 'secret-default');
      assert.equal(props.get_string('fixtureRunId'), RUN_ID);
    } finally {
      props.free();
    }
  } finally {
    clientValid.free();
  }

  // Positive: Valid secret with targeting parameters (signed request)
  const configTargeted = new ClientConfig('101010102', CONFIG_URL, 'default');
  configTargeted.secret = secret;
  configTargeted.ip = '1.2.3.4';
  configTargeted.label = 'GrayScale';
  const clientTargeted = new Client(configTargeted);
  try {
    const props = await clientTargeted.namespace('application');
    try {
      assert.equal(props.get_string('identity'), 'secret-default');
      assert.equal(props.get_bool('grayScaleValue'), true);
      assert.equal(props.get_string('fixtureRunId'), RUN_ID);
    } finally {
      props.free();
    }
  } finally {
    clientTargeted.free();
  }
});

test('real_apollo_wasm_grayscale', async () => {
  // Case 1: Matching IP (1.2.3.4)
  const configIpMatch = new ClientConfig('101010101', CONFIG_URL, 'default');
  configIpMatch.ip = '1.2.3.4';
  const clientIpMatch = new Client(configIpMatch);
  try {
    const props = await clientIpMatch.namespace('application');
    try {
      assert.equal(props.get_bool('grayScaleValue'), true);
      assert.equal(props.get_string('fixtureRunId'), RUN_ID);
    } finally {
      props.free();
    }
  } finally {
    clientIpMatch.free();
  }

  // Case 2: Matching label (GrayScale)
  const configLabelMatch = new ClientConfig('101010101', CONFIG_URL, 'default');
  configLabelMatch.label = 'GrayScale';
  const clientLabelMatch = new Client(configLabelMatch);
  try {
    const props = await clientLabelMatch.namespace('application');
    try {
      assert.equal(props.get_bool('grayScaleValue'), true);
      assert.equal(props.get_string('fixtureRunId'), RUN_ID);
    } finally {
      props.free();
    }
  } finally {
    clientLabelMatch.free();
  }

  // Case 3: Non-matching IP (1.2.3.5)
  const configIpNoMatch = new ClientConfig('101010101', CONFIG_URL, 'default');
  configIpNoMatch.ip = '1.2.3.5';
  const clientIpNoMatch = new Client(configIpNoMatch);
  try {
    const props = await clientIpNoMatch.namespace('application');
    try {
      assert.equal(props.get_bool('grayScaleValue'), false);
      assert.equal(props.get_string('fixtureRunId'), RUN_ID);
    } finally {
      props.free();
    }
  } finally {
    clientIpNoMatch.free();
  }

  // Case 4: Non-matching label (OtherLabel)
  const configLabelNoMatch = new ClientConfig('101010101', CONFIG_URL, 'default');
  configLabelNoMatch.label = 'OtherLabel';
  const clientLabelNoMatch = new Client(configLabelNoMatch);
  try {
    const props = await clientLabelNoMatch.namespace('application');
    try {
      assert.equal(props.get_bool('grayScaleValue'), false);
      assert.equal(props.get_string('fixtureRunId'), RUN_ID);
    } finally {
      props.free();
    }
  } finally {
    clientLabelNoMatch.free();
  }

  // Case 5: Neither IP nor label specified
  const configNeither = new ClientConfig('101010101', CONFIG_URL, 'default');
  const clientNeither = new Client(configNeither);
  try {
    const props = await clientNeither.namespace('application');
    try {
      assert.equal(props.get_bool('grayScaleValue'), false);
      assert.equal(props.get_string('fixtureRunId'), RUN_ID);
    } finally {
      props.free();
    }
  } finally {
    clientNeither.free();
  }
});

test('real_apollo_wasm_release_refresh_and_listener', async () => {
  const appId = '101010101';
  const updateNs = 'updates-wasm';

  // 1. Reset mutable namespace to known baseline
  runFixtureCommand([
    'set-and-publish',
    '--admin-url', ADMIN_URL,
    '--app', appId,
    '--namespace', updateNs,
    '--key', 'value',
    '--value', 'baseline',
    '--name', 'wasm-baseline-reset',
  ]);
  await waitForConfigServiceValue(appId, 'default', updateNs, 'value', 'baseline');

  const config = new ClientConfig(appId, CONFIG_URL, 'default');
  const client = new Client(config);

  try {
    const events = [];
    await client.add_listener(updateNs, (data, error) => {
      if (data && typeof data.value === 'string') {
        events.push(data.value);
      }
    });

    // Initial namespace load: listener receives initial event
    const initialProps = await client.namespace(updateNs);
    try {
      assert.equal(initialProps.get_string('value'), 'baseline');
    } finally {
      initialProps.free();
    }
    assert.deepEqual(events, ['baseline'], 'Initial load must trigger initial listener event');

    // 2. Set item without publishing
    runFixtureCommand([
      'set-item',
      '--admin-url', ADMIN_URL,
      '--app', appId,
      '--namespace', updateNs,
      '--key', 'value',
      '--value', 'edited-unpublished',
    ]);

    // Independent probe sees baseline
    const probeUrl = `${CONFIG_URL.replace(/\/+$/, '')}/configfiles/json/${appId}/default/${updateNs}`;
    const probeResp = await fetch(probeUrl);
    assert.ok(probeResp.ok, 'probe request must succeed');
    const probeData = await probeResp.json();
    assert.equal(probeData.value, 'baseline', 'unpublished item must not be visible on ConfigService');

    // Explicit client refresh sees baseline; listener receives no event
    await client.refresh(updateNs);
    const refreshedProps = await client.namespace(updateNs);
    try {
      assert.equal(refreshedProps.get_string('value'), 'baseline');
    } finally {
      refreshedProps.free();
    }
    assert.deepEqual(events, ['baseline'], 'Unpublished item refresh must not trigger listener event');

    // 3. Publish release
    runFixtureCommand([
      'publish',
      '--admin-url', ADMIN_URL,
      '--app', appId,
      '--namespace', updateNs,
      '--name', 'wasm-publish-edited',
    ]);
    await waitForConfigServiceValue(appId, 'default', updateNs, 'value', 'edited-unpublished');

    // Client refresh now sees published value and listener receives event
    await client.refresh(updateNs);
    const publishedProps = await client.namespace(updateNs);
    try {
      assert.equal(publishedProps.get_string('value'), 'edited-unpublished');
    } finally {
      publishedProps.free();
    }
    assert.deepEqual(
      events,
      ['baseline', 'edited-unpublished'],
      'Published release refresh must trigger listener event'
    );

    // 4. Second refresh with unchanged data must produce no extra listener event
    await client.refresh(updateNs);
    assert.deepEqual(
      events,
      ['baseline', 'edited-unpublished'],
      'Unchanged data refresh must not trigger extra listener event'
    );
  } finally {
    client.free();
  }
});

test('real_apollo_wasm_polling', async () => {
  const appId = '101010101';
  const updateNs = 'updates-wasm';

  // 1. Reset mutable namespace to baseline
  runFixtureCommand([
    'set-and-publish',
    '--admin-url', ADMIN_URL,
    '--app', appId,
    '--namespace', updateNs,
    '--key', 'value',
    '--value', 'baseline',
    '--name', 'wasm-polling-reset',
  ]);
  await waitForConfigServiceValue(appId, 'default', updateNs, 'value', 'baseline');

  const config = new ClientConfig(appId, CONFIG_URL, 'default');
  config.refresh_interval = 1n;
  const client = new Client(config);

  try {
    const events = [];
    await client.add_listener(updateNs, (data, error) => {
      if (data && typeof data.value === 'string') {
        events.push(data.value);
      }
    });

    const initialProps = await client.namespace(updateNs);
    try {
      assert.equal(initialProps.get_string('value'), 'baseline');
    } finally {
      initialProps.free();
    }

    // Start background polling
    client.start();

    // Publish new value to Apollo
    runFixtureCommand([
      'set-and-publish',
      '--admin-url', ADMIN_URL,
      '--app', appId,
      '--namespace', updateNs,
      '--key', 'value',
      '--value', 'polling-published',
      '--name', 'wasm-polling-pub',
    ]);
    await waitForConfigServiceValue(appId, 'default', updateNs, 'value', 'polling-published');

    // Bounded observation: wait for polling background loop to update client without manual refresh
    const start = Date.now();
    const timeoutMs = 60000;
    let observed = false;

    while (Date.now() - start < timeoutMs) {
      if (events.includes('polling-published')) {
        observed = true;
        break;
      }
      try {
        const p = await client.namespace(updateNs);
        const val = p.get_string('value');
        p.free();
        if (val === 'polling-published') {
          observed = true;
          break;
        }
      } catch {
        // continue waiting
      }
      await new Promise((resolve) => setTimeout(resolve, 200));
    }

    assert.ok(observed, 'Background polling loop must observe published update within timeout');
  } finally {
    client.stop();
    client.free();
  }
});

test('real_apollo_wasm_preload', async () => {
  const config = new ClientConfig('101010101', CONFIG_URL, 'default');
  const client = new Client(config);

  try {
    // Preload duplicate namespaces and multiple formats simultaneously
    await client.preload(['application', 'application', 'application.json', 'config', 'application']);

    const appProps = await client.namespace('application');
    try {
      assert.equal(appProps.get_string('stringValue'), 'string value');
      assert.equal(appProps.get_string('fixtureRunId'), RUN_ID);
    } finally {
      appProps.free();
    }

    const jsonObj = await client.namespace('application.json');
    assert.equal(jsonObj.host, 'localhost');
    assert.equal(jsonObj.port, 8080);
    assert.equal(jsonObj.run, true);
  } finally {
    client.free();
  }
});
