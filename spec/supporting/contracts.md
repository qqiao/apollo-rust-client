# External contracts

Status: reconstructed interface reference. The feature specs own user outcomes; this document supplies precise external representations where interoperability requires them. Sources: [configuration](../../src/client_config.rs), [client API](../../src/lib.rs), [retrieval](../../src/cache.rs), and [formats](../../src/namespace/mod.rs).

## Connection configuration

Rust exposes public `ClientConfig` fields, a `ClientConfig::builder(app_id, config_server)` builder, and `ClientConfig::from_env()`. `build()`, `from_env()`, and `Client::new` validate. Public field mutation and the WASM configuration constructor do not themselves validate. Required identifiers are rejected if whitespace-only; nonempty values are not trimmed. The server must parse as an HTTP(S) base URL without query/fragment.

| Field | Environment variable | Default / validation |
|---|---|---|
| `app_id` | `APP_ID` | Required, nonblank |
| `config_server` | `APOLLO_CONFIG_SERVICE` | Required HTTP(S) base |
| `cluster` | `IDC` | `default`, nonblank |
| `secret` | `APOLLO_ACCESS_KEY_SECRET` | Absent |
| `label` | `APOLLO_LABEL` | Absent |
| `ip` | No mapping | Absent; direct/builder setting |
| `cache_dir` | `APOLLO_CACHE_DIR` | Platform default on native; ignored on WASM |
| `allow_insecure_https` | `APOLLO_ALLOW_INSECURE_HTTPS` | Absent means false; environment parses `true`/`false` |
| `cache_ttl` | `APOLLO_CACHE_TTL` | 600 seconds; zero accepted |
| `refresh_interval` | `APOLLO_REFRESH_INTERVAL` | 30 seconds; zero rejected |
| `request_timeout` | `APOLLO_REQUEST_TIMEOUT` | 10 seconds; zero rejected |
| `http_client` | No mapping | Optional native-only `reqwest::Client` |

Timing environment values parse as `u64`; omitted/`None` timings use effective defaults. Builder setters cover every optional setting and cluster; build validates. Native environment access reports missing/non-Unicode values. Node WASM reads `globalThis.process.env`; inaccessible/non-string values fail. Browser-style environments without process variables cannot supply required environment settings.

## Rust consumer boundary

| Operation | Signature / semantics |
|---|---|
| Construction | `Client::new(config: ClientConfig) -> Result<Client, Error>` |
| Read | `namespace(&self, namespace: &str).await -> Result<Namespace, Error>` |
| Register listener | `add_listener(&self, namespace: &str, listener: EventListener).await -> ()` |
| Warm namespaces | `preload(&self, namespaces: &[impl AsRef<str>]).await -> Result<(), Error>` |
| Remote refresh | `refresh(&self, namespace: &str).await -> Result<(), Error>` |
| Native lifecycle | `start(&mut self).await -> Result<(), Error>` and `stop(&mut self).await -> ()` |

Native `EventListener` is `Arc<dyn Fn(Result<Namespace, Error>) + Send + Sync>`; WASM Rust listeners omit `Send + Sync`. Public `Namespace` variants are Properties, Json, Yaml, Text. Public format helpers are in `namespace::{properties,json,yaml}`; cache internals are not a public consumer API.

Properties provides `get_property<T: FromStr>` and string/i64/f64/bool convenience getters returning `Option<T>`. JSON `to_object<T: DeserializeOwned>` returns serde_json errors; YAML `to_object<T: DeserializeOwned + 'static>` returns noyalib errors. These conversions are synchronous.

## Apollo wire boundary

The client sends GET to the configured base path with `/configfiles/json/{app_id}/{cluster}/{namespace}` appended using URL segment encoding. Optional query pairs are `ip`, then `label`. The outer response is JSON. Properties uses that value directly; JSON/YAML/Text expects a string under `content`, interpreted according to the namespace suffix. Unknown suffixes use Properties; XML is unsupported.

When a secret is set:

```text
timestamp: <Unix time in milliseconds>
Authorization: Apollo <app_id>:<signature>
signature = Base64(HMAC-SHA1(secret, timestamp + "\n" + encoded_path_and_query))
```

The signature covers the actual encoded request path and query. Existing signing test vectors use `/configs/...` as a sample signing input; retrieval itself uses `/configfiles/json/...`. Successful HTTP status plus parseable outer JSON is a retrieval success; format validation is a later layer. No release-key, server-notification, or conditional-GET protocol is implemented.

## JavaScript consumer boundary

Configuration construction is `new ClientConfig(appId, configServer, cluster)`; `new Client(config)` consumes that configuration wrapper and validates it. Public fields have generated accessors except native `http_client`. Timing fields and signed Properties integers use wasm-bindgen's bigint mapping. The generated declarations govern precise optional unions; no handwritten null-only return signature is asserted here.

Client exports `namespace`, `add_listener`, `preload`, `refresh`, `start`, `stop`. The first four are promise-based; the last two are synchronous. Preload accepts an array and rejects any non-string entry before reads. Properties getters are synchronous. Direct Properties reads return owned wrappers; listeners receive plain serialized Properties data. JSON/YAML values and Text are ordinary JS-managed values.

Callbacks receive `(data, error)`: the inapplicable argument is `undefined`. Caught exceptions are logged. Live Client/Properties wrappers have `.free()`; transfer to Client consumes the configuration wrapper, so callers must not reuse it or treat it as still owned. An unconsumed configuration wrapper remains the caller's to release. Generated-code ownership behavior has not been re-executed in this documentation task; see traceability.

## Error contract and limitations

| Boundary | Error representation |
|---|---|
| Configuration | `EnvVar(error, name)` or `InvalidValue { name, value, reason }` |
| Client operations | `AlreadyRunning`, `Config`, `HttpClient`, `Cache`, `Namespace`, `Refresh(String)` |
| Retrieval internals | Transport (`Reqwest`), `HttpStatus { status, body }`, `Serde`, `Timeout { seconds }`, URL/signing failures; coalesced followers can receive string snapshots |
| Native namespace conversion | JSON/YAML content/conversion errors, Text content error, unsupported XML error |
| WASM Client operation failure | JavaScript `Error` object |
| WASM direct configuration error conversion | String |
| WASM value conversion failure | Logged null fallback in current helpers; see research decision D-002 |

The request deadline bounds asynchronous network waiting, including body reading. It does not cover mutex waits, synchronous parsing monopolizing the executor, persistence, or callbacks as an overall operation deadline. The native custom transport may return its own error before that deadline. Error details/logs are not a declared redaction or payload-size policy.
