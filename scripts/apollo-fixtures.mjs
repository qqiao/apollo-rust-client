#!/usr/bin/env node
/**
 * Apollo test fixture initialization and verification tool.
 * Manages test apps, clusters, namespaces, items, and releases via AdminService,
 * and validates published values via ConfigService.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const DEFAULT_FIXTURES_PATH = path.resolve(__dirname, '../tests/apollo/fixtures.json');

const AUDIT_USER = 'apollo-test';
const REQUEST_TIMEOUT_MS = 5000;

function validateLoopbackUrl(urlString, name) {
  if (!urlString) {
    throw new Error(`Missing required parameter: ${name}`);
  }
  let parsed;
  try {
    parsed = new URL(urlString);
  } catch (err) {
    throw new Error(`Invalid URL for ${name}: ${urlString} (${err.message})`);
  }
  if (parsed.protocol !== 'http:') {
    throw new Error(`Insecure or unsupported protocol for ${name}: ${parsed.protocol} (must be http:)`);
  }
  const host = parsed.hostname;
  if (host !== '127.0.0.1' && host !== 'localhost') {
    throw new Error(`Non-loopback URL rejected for ${name}: ${host} (must be 127.0.0.1 or localhost)`);
  }
  return parsed.origin;
}

async function fetchWithTimeout(url, options = {}) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), options.timeoutMs || REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal,
    });
    return response;
  } finally {
    clearTimeout(timeout);
  }
}

async function requestJson(url, options = {}) {
  const headers = {
    Accept: 'application/json',
    ...(options.headers || {}),
  };
  if (options.body && typeof options.body === 'object') {
    headers['Content-Type'] = 'application/json';
    options.body = JSON.stringify(options.body);
  }
  const response = await fetchWithTimeout(url, { ...options, headers });
  const text = await response.text();
  let data = null;
  if (text.length > 0) {
    try {
      data = JSON.parse(text);
    } catch {
      data = text;
    }
  }
  return { status: response.status, headers: response.headers, data, ok: response.ok };
}

// ---------------- AdminService Reconcile Helpers ----------------

async function ensureApp(adminUrl, app) {
  const getRes = await requestJson(`${adminUrl}/apps/${encodeURIComponent(app.appId)}`);
  if (getRes.status === 200 && getRes.data && getRes.data.appId === app.appId) {
    return getRes.data;
  }
  if (getRes.status !== 404) {
    throw new Error(`Failed checking app ${app.appId}: HTTP ${getRes.status}: ${JSON.stringify(getRes.data)}`);
  }
  const createRes = await requestJson(`${adminUrl}/apps`, {
    method: 'POST',
    body: {
      appId: app.appId,
      name: app.name,
      orgId: app.orgId,
      orgName: app.orgName,
      ownerName: app.ownerName,
      ownerEmail: app.ownerEmail || 'apollo-test@example.com',
      dataChangeCreatedBy: AUDIT_USER,
      dataChangeLastModifiedBy: AUDIT_USER,
    },
  });
  if (!createRes.ok) {
    throw new Error(`Failed creating app ${app.appId}: HTTP ${createRes.status}: ${JSON.stringify(createRes.data)}`);
  }
  return createRes.data;
}

async function ensureCluster(adminUrl, appId, clusterName) {
  const getRes = await requestJson(`${adminUrl}/apps/${encodeURIComponent(appId)}/clusters/${encodeURIComponent(clusterName)}`);
  if (getRes.status === 200 && getRes.data && getRes.data.name === clusterName) {
    return getRes.data;
  }
  if (getRes.status !== 404) {
    throw new Error(`Failed checking cluster ${appId}/${clusterName}: HTTP ${getRes.status}: ${JSON.stringify(getRes.data)}`);
  }
  const createRes = await requestJson(`${adminUrl}/apps/${encodeURIComponent(appId)}/clusters`, {
    method: 'POST',
    body: {
      appId,
      name: clusterName,
      comment: '',
      dataChangeCreatedBy: AUDIT_USER,
      dataChangeLastModifiedBy: AUDIT_USER,
    },
  });
  if (!createRes.ok) {
    throw new Error(`Failed creating cluster ${appId}/${clusterName}: HTTP ${createRes.status}: ${JSON.stringify(createRes.data)}`);
  }
  return createRes.data;
}

async function ensureAppNamespace(adminUrl, appId, nsDef) {
  if (nsDef.name === 'application' && nsDef.format === 'properties' && !nsDef.isPublic) {
    return null; // default application namespace is automatically created
  }
  const listRes = await requestJson(`${adminUrl}/apps/${encodeURIComponent(appId)}/appnamespaces`);
  if (!listRes.ok) {
    throw new Error(`Failed listing app namespaces for ${appId}: HTTP ${listRes.status}`);
  }
  const existing = Array.isArray(listRes.data) && listRes.data.find(n => n.name === nsDef.name);
  if (existing) {
    return existing;
  }
  const createRes = await requestJson(`${adminUrl}/apps/${encodeURIComponent(appId)}/appnamespaces`, {
    method: 'POST',
    body: {
      appId,
      name: nsDef.name,
      format: nsDef.format,
      isPublic: !!nsDef.isPublic,
      comment: '',
      dataChangeCreatedBy: AUDIT_USER,
      dataChangeLastModifiedBy: AUDIT_USER,
    },
  });
  if (!createRes.ok) {
    throw new Error(`Failed creating app namespace ${appId}/${nsDef.name}: HTTP ${createRes.status}: ${JSON.stringify(createRes.data)}`);
  }
  return createRes.data;
}

async function ensureAssociatedNamespace(adminUrl, consumerAppId, clusterName, publicNsName) {
  const getRes = await requestJson(
    `${adminUrl}/apps/${encodeURIComponent(consumerAppId)}/clusters/${encodeURIComponent(clusterName)}/namespaces/${encodeURIComponent(publicNsName)}/associated-public-namespace`
  );
  if (getRes.status === 200) {
    return getRes.data;
  }
  const createRes = await requestJson(
    `${adminUrl}/apps/${encodeURIComponent(consumerAppId)}/clusters/${encodeURIComponent(clusterName)}/namespaces`,
    {
      method: 'POST',
      body: {
        appId: consumerAppId,
        clusterName,
        namespaceName: publicNsName,
        dataChangeCreatedBy: AUDIT_USER,
        dataChangeLastModifiedBy: AUDIT_USER,
      },
    }
  );
  if (!createRes.ok) {
    throw new Error(`Failed associating namespace ${consumerAppId}/${publicNsName}: HTTP ${createRes.status}: ${JSON.stringify(createRes.data)}`);
  }
  return createRes.data;
}

async function getNamespace(adminUrl, appId, clusterName, namespaceName) {
  const res = await requestJson(
    `${adminUrl}/apps/${encodeURIComponent(appId)}/clusters/${encodeURIComponent(clusterName)}/namespaces/${encodeURIComponent(namespaceName)}`
  );
  if (!res.ok) {
    throw new Error(`Failed resolving namespace ${appId}/${clusterName}/${namespaceName}: HTTP ${res.status}: ${JSON.stringify(res.data)}`);
  }
  return res.data;
}

async function reconcileItems(adminUrl, appId, clusterName, namespaceName, namespaceId, desiredItems) {
  const itemsRes = await requestJson(
    `${adminUrl}/apps/${encodeURIComponent(appId)}/clusters/${encodeURIComponent(clusterName)}/namespaces/${encodeURIComponent(namespaceName)}/items`
  );
  if (!itemsRes.ok) {
    throw new Error(`Failed fetching items for ${appId}/${clusterName}/${namespaceName}: HTTP ${itemsRes.status}`);
  }
  const existingItems = Array.isArray(itemsRes.data) ? itemsRes.data : [];
  const existingMap = new Map(existingItems.filter(i => i.key).map(i => [i.key, i]));

  for (const [key, val] of Object.entries(desiredItems)) {
    const stringVal = String(val);
    const existing = existingMap.get(key);
    if (existing) {
      if (existing.value !== stringVal) {
        const updateRes = await requestJson(
          `${adminUrl}/apps/${encodeURIComponent(appId)}/clusters/${encodeURIComponent(clusterName)}/namespaces/${encodeURIComponent(namespaceName)}/items/${existing.id}`,
          {
            method: 'PUT',
            body: {
              id: existing.id,
              namespaceId,
              key,
              value: stringVal,
              comment: '',
              dataChangeCreatedBy: AUDIT_USER,
              dataChangeLastModifiedBy: AUDIT_USER,
            },
          }
        );
        if (!updateRes.ok) {
          throw new Error(`Failed updating item ${key} in ${appId}/${clusterName}/${namespaceName}: HTTP ${updateRes.status}: ${JSON.stringify(updateRes.data)}`);
        }
      }
    } else {
      const createRes = await requestJson(
        `${adminUrl}/apps/${encodeURIComponent(appId)}/clusters/${encodeURIComponent(clusterName)}/namespaces/${encodeURIComponent(namespaceName)}/items`,
        {
          method: 'POST',
          body: {
            namespaceId,
            key,
            value: stringVal,
            comment: '',
            dataChangeCreatedBy: AUDIT_USER,
            dataChangeLastModifiedBy: AUDIT_USER,
          },
        }
      );
      if (!createRes.ok) {
        throw new Error(`Failed creating item ${key} in ${appId}/${clusterName}/${namespaceName}: HTTP ${createRes.status}: ${JSON.stringify(createRes.data)}`);
      }
    }
  }
}

async function reconcileRelease(adminUrl, appId, clusterName, namespaceName, desiredItems, releaseNamePrefix) {
  const latestRes = await requestJson(
    `${adminUrl}/apps/${encodeURIComponent(appId)}/clusters/${encodeURIComponent(clusterName)}/namespaces/${encodeURIComponent(namespaceName)}/releases/latest`
  );

  let needPublish = true;
  if (latestRes.ok && latestRes.data && latestRes.data.configurations) {
    try {
      const activeConfigs = JSON.parse(latestRes.data.configurations);
      const desiredKeys = Object.keys(desiredItems);
      const activeKeys = Object.keys(activeConfigs);
      if (desiredKeys.length === activeKeys.length) {
        const allMatch = desiredKeys.every(k => String(desiredItems[k]) === String(activeConfigs[k]));
        if (allMatch) {
          needPublish = false;
          return { published: false, release: latestRes.data };
        }
      }
    } catch {
      needPublish = true;
    }
  }

  if (needPublish) {
    const title = `${releaseNamePrefix}-${Date.now()}`;
    const pubUrl = `${adminUrl}/apps/${encodeURIComponent(appId)}/clusters/${encodeURIComponent(clusterName)}/namespaces/${encodeURIComponent(namespaceName)}/releases?name=${encodeURIComponent(title)}&operator=${encodeURIComponent(AUDIT_USER)}&isEmergencyPublish=false`;
    const pubRes = await requestJson(pubUrl, { method: 'POST' });
    if (!pubRes.ok) {
      throw new Error(`Failed publishing release for ${appId}/${clusterName}/${namespaceName}: HTTP ${pubRes.status}: ${JSON.stringify(pubRes.data)}`);
    }
    return { published: true, release: pubRes.data };
  }
}

// ---------------- Operations ----------------

export async function seedFixtures({ adminUrl, configUrl, runId, fixturesPath, statePath }) {
  const fixturesRaw = fs.readFileSync(fixturesPath, 'utf8');
  const fixtures = JSON.parse(fixturesRaw);
  const resolvedState = {
    schemaVersion: 1,
    runId,
    timestamp: new Date().toISOString(),
    apps: {},
  };

  for (const app of fixtures.apps) {
    const appEntity = await ensureApp(adminUrl, app);
    resolvedState.apps[app.appId] = {
      id: appEntity.id,
      name: app.name,
      clusters: {},
    };

    // Clusters
    if (app.clusters) {
      for (const cl of app.clusters) {
        await ensureCluster(adminUrl, app.appId, cl.name);
      }
    }

    // App Namespaces
    if (app.namespaces) {
      for (const ns of app.namespaces) {
        await ensureAppNamespace(adminUrl, app.appId, ns);
      }
    }

    // Associated Namespaces
    if (app.associatedNamespaces) {
      for (const assoc of app.associatedNamespaces) {
        await ensureAssociatedNamespace(adminUrl, app.appId, assoc.cluster, assoc.namespaceName);
      }
    }

    // Items and Releases for Namespaces
    if (app.namespaces) {
      for (const ns of app.namespaces) {
        const cluster = ns.cluster || 'default';
        const nsEntity = await getNamespace(adminUrl, app.appId, cluster, ns.name);

        let desiredItems = {};
        if (ns.format === 'properties') {
          desiredItems = { ...(ns.items || {}), fixtureRunId: runId };
        } else {
          desiredItems = { content: ns.content || '' };
        }

        await reconcileItems(adminUrl, app.appId, cluster, ns.name, nsEntity.id, desiredItems);
        const relResult = await reconcileRelease(
          adminUrl,
          app.appId,
          cluster,
          ns.name,
          desiredItems,
          `seed-${ns.name}`
        );

        if (!resolvedState.apps[app.appId].clusters[cluster]) {
          resolvedState.apps[app.appId].clusters[cluster] = { namespaces: {} };
        }
        resolvedState.apps[app.appId].clusters[cluster].namespaces[ns.name] = {
          id: nsEntity.id,
          format: ns.format,
          latestReleaseId: relResult.release ? relResult.release.id : null,
        };
      }
    }
  }

  if (statePath) {
    fs.mkdirSync(path.dirname(statePath), { recursive: true });
    fs.writeFileSync(statePath, JSON.stringify(resolvedState, null, 2), 'utf8');
  }

  return resolvedState;
}

export async function verifyFixtures({ configUrl, runId, fixturesPath }) {
  const fixturesRaw = fs.readFileSync(fixturesPath, 'utf8');
  const fixtures = JSON.parse(fixturesRaw);

  const verificationResults = [];

  for (const app of fixtures.apps) {
    if (!app.namespaces) continue;
    for (const ns of app.namespaces) {
      const cluster = ns.cluster || 'default';
      const probeUrl = `${configUrl}/configfiles/json/${encodeURIComponent(app.appId)}/${encodeURIComponent(cluster)}/${encodeURIComponent(ns.name)}`;

      let lastError = null;
      let ok = false;
      // Retry probe up to 10 times with 500ms sleep in case of release propagation
      for (let attempt = 1; attempt <= 10; attempt++) {
        try {
          const res = await requestJson(probeUrl);
          if (res.status === 200 && res.data) {
            if (ns.format === 'properties') {
              const expectedItems = { ...(ns.items || {}), fixtureRunId: runId };
              for (const [k, v] of Object.entries(expectedItems)) {
                if (String(res.data[k]) !== String(v)) {
                  throw new Error(`Namespace ${app.appId}/${cluster}/${ns.name} key "${k}": expected "${v}", got "${res.data[k]}"`);
                }
              }
              if (res.data.missingValue !== undefined) {
                throw new Error(`Namespace ${app.appId}/${cluster}/${ns.name} contained unexpected "missingValue"`);
              }
            } else {
              // Non-properties namespace returns JSON wrapper or parsed content
              if (res.data.content === undefined) {
                throw new Error(`Namespace ${app.appId}/${cluster}/${ns.name} missing "content" field in response: ${JSON.stringify(res.data)}`);
              }
              if (res.data.content !== ns.content) {
                throw new Error(`Namespace ${app.appId}/${cluster}/${ns.name} content mismatch.\nExpected: ${JSON.stringify(ns.content)}\nGot: ${JSON.stringify(res.data.content)}`);
              }
            }
            ok = true;
            break;
          } else {
            throw new Error(`HTTP ${res.status}: ${JSON.stringify(res.data)}`);
          }
        } catch (err) {
          lastError = err;
          await new Promise(r => setTimeout(r, 500));
        }
      }

      if (!ok) {
        throw new Error(`Verification failed for ${app.appId}/${cluster}/${ns.name}: ${lastError ? lastError.message : 'unknown error'}`);
      }
      verificationResults.push({ appId: app.appId, cluster, namespace: ns.name, status: 'OK' });

      // If properties format, also verify query with .properties suffix behaves identically
      if (ns.format === 'properties' && !ns.name.endsWith('.properties')) {
        const suffixedUrl = `${configUrl}/configfiles/json/${encodeURIComponent(app.appId)}/${encodeURIComponent(cluster)}/${encodeURIComponent(ns.name)}.properties`;
        const sufRes = await requestJson(suffixedUrl);
        if (!sufRes.ok || !sufRes.data) {
          throw new Error(`Suffixed verification failed for ${app.appId}/${cluster}/${ns.name}.properties: HTTP ${sufRes.status}`);
        }
        const expectedItems = { ...(ns.items || {}), fixtureRunId: runId };
        for (const [k, v] of Object.entries(expectedItems)) {
          if (String(sufRes.data[k]) !== String(v)) {
            throw new Error(`Suffixed namespace ${app.appId}/${cluster}/${ns.name}.properties key "${k}": expected "${v}", got "${sufRes.data[k]}"`);
          }
        }
        verificationResults.push({ appId: app.appId, cluster, namespace: `${ns.name}.properties`, status: 'OK (suffixed)' });
      }
    }

    // Also verify associated public namespace for consumer apps
    if (app.associatedNamespaces) {
      for (const assoc of app.associatedNamespaces) {
        const cluster = assoc.cluster || 'default';
        const probeUrl = `${configUrl}/configfiles/json/${encodeURIComponent(app.appId)}/${encodeURIComponent(cluster)}/${encodeURIComponent(assoc.namespaceName)}`;

        let lastError = null;
        let ok = false;
        for (let attempt = 1; attempt <= 10; attempt++) {
          try {
            const res = await requestJson(probeUrl);
            if (res.ok && res.data) {
              if (res.data.publicValue !== 'associated') {
                throw new Error(`Associated namespace ${app.appId}/${cluster}/${assoc.namespaceName} expected publicValue="associated", got "${res.data.publicValue}"`);
              }
              ok = true;
              break;
            } else {
              throw new Error(`HTTP ${res.status}: ${JSON.stringify(res.data)}`);
            }
          } catch (err) {
            lastError = err;
            await new Promise(r => setTimeout(r, 500));
          }
        }

        if (!ok) {
          throw new Error(`Verification failed for associated namespace ${app.appId}/${cluster}/${assoc.namespaceName}: ${lastError ? lastError.message : 'unknown error'}`);
        }
        verificationResults.push({ appId: app.appId, cluster, namespace: assoc.namespaceName, status: 'OK (associated)' });
      }
    }
  }

  // Negative control: Nonexistent namespace must return 404
  const negProbeUrl = `${configUrl}/configfiles/json/101010101/default/nonexistent.namespace`;
  const negRes = await requestJson(negProbeUrl);
  if (negRes.status !== 404) {
    throw new Error(`Negative control failed: expected HTTP 404 for nonexistent namespace, got ${negRes.status}`);
  }
  verificationResults.push({ probe: 'nonexistent.namespace', status: 'OK (404 Not Found)' });

  return verificationResults;
}

export async function setItem({ adminUrl, appId, clusterName, namespaceName, key, value, fixturesPath }) {
  const fixturesRaw = fs.readFileSync(fixturesPath, 'utf8');
  const fixtures = JSON.parse(fixturesRaw);
  const appDef = fixtures.apps.find(a => a.appId === appId);
  if (!appDef) {
    throw new Error(`App ${appId} not found in fixtures`);
  }
  const nsDef = appDef.namespaces && appDef.namespaces.find(n => (n.cluster || 'default') === clusterName && n.name === namespaceName);
  if (!nsDef || !nsDef.mutable) {
    throw new Error(`Mutation rejected: namespace ${appId}/${clusterName}/${namespaceName} is not declared mutable in fixtures`);
  }

  const nsEntity = await getNamespace(adminUrl, appId, clusterName, namespaceName);
  await reconcileItems(adminUrl, appId, clusterName, namespaceName, nsEntity.id, { [key]: value });
  return { appId, clusterName, namespaceName, key, value, updated: true };
}

export async function publishRelease({ adminUrl, appId, clusterName, namespaceName, releaseName, fixturesPath }) {
  const fixturesRaw = fs.readFileSync(fixturesPath, 'utf8');
  const fixtures = JSON.parse(fixturesRaw);
  const appDef = fixtures.apps.find(a => a.appId === appId);
  if (!appDef) {
    throw new Error(`App ${appId} not found in fixtures`);
  }
  const nsDef = appDef.namespaces && appDef.namespaces.find(n => (n.cluster || 'default') === clusterName && n.name === namespaceName);
  if (!nsDef || !nsDef.mutable) {
    throw new Error(`Mutation rejected: namespace ${appId}/${clusterName}/${namespaceName} is not declared mutable in fixtures`);
  }

  const title = releaseName || `release-${Date.now()}`;
  const pubUrl = `${adminUrl}/apps/${encodeURIComponent(appId)}/clusters/${encodeURIComponent(clusterName)}/namespaces/${encodeURIComponent(namespaceName)}/releases?name=${encodeURIComponent(title)}&operator=${encodeURIComponent(AUDIT_USER)}&isEmergencyPublish=false`;
  const pubRes = await requestJson(pubUrl, { method: 'POST' });
  if (!pubRes.ok) {
    throw new Error(`Failed publishing release for ${appId}/${clusterName}/${namespaceName}: HTTP ${pubRes.status}: ${JSON.stringify(pubRes.data)}`);
  }
  return pubRes.data;
}

// ---------------- CLI Entry Point ----------------

function parseArgs(args) {
  const parsed = {
    command: args[0],
    options: {},
  };
  for (let i = 1; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      const next = args[i + 1];
      if (next && !next.startsWith('--')) {
        parsed.options[key] = next;
        i++;
      } else {
        parsed.options[key] = true;
      }
    }
  }
  return parsed;
}

async function main() {
  const { command, options } = parseArgs(process.argv.slice(2));

  if (!command || command === 'help' || command === '--help') {
    console.log(`Usage: apollo-fixtures.mjs <command> [options]
Commands:
  seed             Initialize all apps, clusters, namespaces, items, and publish releases
  verify           Verify published values through ConfigService
  set-item         Set or update an item in a mutable namespace (without publishing)
  publish          Publish a release in a mutable namespace
  set-and-publish  Set item and publish release
Options:
  --admin-url      AdminService URL (e.g. http://127.0.0.1:8090)
  --config-url     ConfigService URL (e.g. http://127.0.0.1:8080)
  --run-id         Run marker string
  --fixtures       Path to fixtures.json
  --state-file     Path to output state.json
  --app            App ID for mutation
  --cluster        Cluster name for mutation
  --namespace      Namespace name for mutation
  --key            Item key for mutation
  --value          Item value for mutation
  --name           Release name
`);
    process.exit(0);
  }

  const fixturesPath = path.resolve(process.cwd(), options.fixtures || DEFAULT_FIXTURES_PATH);
  const statePath = options['state-file'] ? path.resolve(process.cwd(), options['state-file']) : null;

  try {
    if (command === 'seed') {
      const adminUrl = validateLoopbackUrl(options['admin-url'], '--admin-url');
      const configUrl = validateLoopbackUrl(options['config-url'], '--config-url');
      const runId = options['run-id'] || `run-${Date.now()}`;
      const state = await seedFixtures({ adminUrl, configUrl, runId, fixturesPath, statePath });
      console.log(JSON.stringify({ status: 'SUCCESS', action: 'seed', runId, state }, null, 2));
    } else if (command === 'verify') {
      const configUrl = validateLoopbackUrl(options['config-url'], '--config-url');
      const runId = options['run-id'] || '';
      const results = await verifyFixtures({ configUrl, runId, fixturesPath });
      console.log(JSON.stringify({ status: 'SUCCESS', action: 'verify', results }, null, 2));
    } else if (command === 'set-item') {
      const adminUrl = validateLoopbackUrl(options['admin-url'], '--admin-url');
      const appId = options.app;
      const clusterName = options.cluster || 'default';
      const namespaceName = options.namespace;
      const key = options.key;
      const value = options.value;
      if (!appId || !namespaceName || !key || value === undefined) {
        throw new Error('set-item requires --app, --namespace, --key, and --value');
      }
      const res = await setItem({ adminUrl, appId, clusterName, namespaceName, key, value, fixturesPath });
      console.log(JSON.stringify({ status: 'SUCCESS', action: 'set-item', result: res }, null, 2));
    } else if (command === 'publish') {
      const adminUrl = validateLoopbackUrl(options['admin-url'], '--admin-url');
      const appId = options.app;
      const clusterName = options.cluster || 'default';
      const namespaceName = options.namespace;
      const releaseName = options.name;
      if (!appId || !namespaceName) {
        throw new Error('publish requires --app and --namespace');
      }
      const res = await publishRelease({ adminUrl, appId, clusterName, namespaceName, releaseName, fixturesPath });
      console.log(JSON.stringify({ status: 'SUCCESS', action: 'publish', release: res }, null, 2));
    } else if (command === 'set-and-publish') {
      const adminUrl = validateLoopbackUrl(options['admin-url'], '--admin-url');
      const appId = options.app;
      const clusterName = options.cluster || 'default';
      const namespaceName = options.namespace;
      const key = options.key;
      const value = options.value;
      const releaseName = options.name;
      if (!appId || !namespaceName || !key || value === undefined) {
        throw new Error('set-and-publish requires --app, --namespace, --key, and --value');
      }
      await setItem({ adminUrl, appId, clusterName, namespaceName, key, value, fixturesPath });
      const pubRes = await publishRelease({ adminUrl, appId, clusterName, namespaceName, releaseName, fixturesPath });
      console.log(JSON.stringify({ status: 'SUCCESS', action: 'set-and-publish', release: pubRes }, null, 2));
    } else {
      throw new Error(`Unknown command: ${command}`);
    }
  } catch (err) {
    console.error(`ERROR [${command}]: ${err.message}`);
    process.exit(1);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
