//! In-process HTTPS support shared by native tests.

use rcgen::generate_simple_self_signed;
use rustls::{
    ServerConfig, ServerConnection, StreamOwned,
    pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer},
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

/// Response returned by a [`MockHttpsServer`] handler.
#[derive(Clone, Debug)]
pub(crate) struct MockResponse {
    pub(crate) status: u16,
    pub(crate) body: String,
    pub(crate) header_delay: Duration,
    pub(crate) body_delay: Duration,
}

impl MockResponse {
    pub(crate) fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            header_delay: Duration::ZERO,
            body_delay: Duration::ZERO,
        }
    }

    pub(crate) fn delayed_body(mut self, delay: Duration) -> Self {
        self.body_delay = delay;
        self
    }
}

pub(crate) type ResponseHandler = dyn Fn(usize, &str) -> MockResponse + Send + Sync + 'static;

/// A random-port, self-signed HTTPS server for transport-level client tests.
pub(crate) struct MockHttpsServer {
    address: SocketAddr,
    requests: Arc<AtomicUsize>,
    captured: Arc<Mutex<Vec<String>>>,
    running: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl MockHttpsServer {
    pub(crate) fn new(handler: Arc<ResponseHandler>) -> Self {
        let certified = generate_simple_self_signed(vec!["localhost".to_string()])
            .expect("test certificate generation should succeed");
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            certified.signing_key.serialize_der(),
        ));
        let tls_config = Arc::new(
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(vec![certified.cert.der().clone()], private_key)
                .expect("test TLS configuration should be valid"),
        );

        let listener = TcpListener::bind("127.0.0.1:0")
            .expect("test HTTPS server should bind to a random local port");
        listener
            .set_nonblocking(true)
            .expect("test listener should support nonblocking mode");
        let address = listener
            .local_addr()
            .expect("test listener should have a local address");
        let requests = Arc::new(AtomicUsize::new(0));
        let captured = Arc::new(Mutex::new(Vec::new()));
        let running = Arc::new(AtomicBool::new(true));

        let requests_in_thread = requests.clone();
        let captured_in_thread = captured.clone();
        let running_in_thread = running.clone();
        let thread = thread::spawn(move || {
            while running_in_thread.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let handler = handler.clone();
                        let tls_config = tls_config.clone();
                        let requests = requests_in_thread.clone();
                        let captured = captured_in_thread.clone();
                        thread::spawn(move || {
                            Self::respond(
                                stream,
                                tls_config,
                                handler.as_ref(),
                                &requests,
                                &captured,
                            );
                        });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            address,
            requests,
            captured,
            running,
            thread: Some(thread),
        }
    }

    fn respond(
        stream: TcpStream,
        tls_config: Arc<ServerConfig>,
        handler: &ResponseHandler,
        requests: &AtomicUsize,
        captured: &Mutex<Vec<String>>,
    ) {
        let _ = stream.set_nonblocking(false);
        let Ok(connection) = ServerConnection::new(tls_config) else {
            return;
        };
        let mut stream = StreamOwned::new(connection, stream);
        let _ = stream.sock.set_read_timeout(Some(Duration::from_secs(2)));
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
            }
        }

        let request = String::from_utf8_lossy(&request).into_owned();
        captured
            .lock()
            .expect("captured-request lock should not be poisoned")
            .push(request.clone());
        let index = requests.fetch_add(1, Ordering::AcqRel) + 1;
        let response = handler(index, &request);
        thread::sleep(response.header_delay);

        let reason = if (200..300).contains(&response.status) {
            "OK"
        } else {
            "Error"
        };
        let headers = format!(
            "HTTP/1.1 {} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.status,
            response.body.len()
        );
        if stream.write_all(headers.as_bytes()).is_err() || stream.flush().is_err() {
            return;
        }
        thread::sleep(response.body_delay);
        let _ = stream.write_all(response.body.as_bytes());
        let _ = stream.flush();
    }

    pub(crate) fn url(&self) -> String {
        format!("https://localhost:{}", self.address.port())
    }

    pub(crate) fn request_count(&self) -> usize {
        self.requests.load(Ordering::Acquire)
    }

    pub(crate) fn request_count_for_path(&self, path: &str) -> usize {
        self.captured
            .lock()
            .expect("captured-request lock should not be poisoned")
            .iter()
            .filter(|request| {
                request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .is_some_and(|request_path| request_path.split('?').next() == Some(path))
            })
            .count()
    }

    pub(crate) fn captured_requests(&self) -> Vec<String> {
        self.captured
            .lock()
            .expect("captured-request lock should not be poisoned")
            .clone()
    }

    pub(crate) async fn wait_for_requests(&self, expected: usize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while self.request_count() < expected {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("server did not receive the expected request count");
    }
}

impl Drop for MockHttpsServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) fn apollo_server() -> &'static MockHttpsServer {
    static SERVER: OnceLock<MockHttpsServer> = OnceLock::new();
    SERVER.get_or_init(|| MockHttpsServer::new(Arc::new(|_, request| apollo_response(request))))
}

fn apollo_response(request: &str) -> MockResponse {
    let path_and_query = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or_default();
    let path = path_and_query.split('?').next().unwrap_or_default();

    if path.contains("/http-401/") {
        return MockResponse::json(401, r#"{"message":"unauthorized"}"#);
    }
    if path.contains("/http-429/") {
        return MockResponse::json(429, r#"{"message":"rate limited"}"#);
    }
    if path.contains("/http-500/") {
        return MockResponse::json(500, r#"{"message":"internal error"}"#);
    }
    if path.contains("/malformed/") {
        return MockResponse::json(200, "{not valid json");
    }
    if path.contains("/timeout/") {
        return MockResponse::json(200, r#"{"value":"too late"}"#)
            .delayed_body(Duration::from_secs(2));
    }

    let body = match path.rsplit('/').next().unwrap_or_default() {
        "application" => {
            let grayscale =
                path_and_query.contains("ip=1.2.3.4") || path_and_query.contains("label=GrayScale");
            format!(
                r#"{{"stringValue":"string value","intValue":"42","floatValue":"4.20","boolValue":"false","grayScaleValue":"{grayscale}"}}"#
            )
        }
        "application.json" => {
            r#"{"content":"{\n  \"host\": \"localhost\",\n  \"port\": 8080,\n  \"run\": true\n}"}"#
                .to_string()
        }
        "application.yml" | "application.yaml" => {
            r#"{"content":"host: \"localhost\"\nport: 8080\nrun: true"}"#.to_string()
        }
        "config.properties" => r#"{"publicValue":"properties"}"#.to_string(),
        "FX.apollo" => r#"{"publicValue":"associated"}"#.to_string(),
        "readme.txt" => r#"{"content":"plain text configuration"}"#.to_string(),
        _ => return MockResponse::json(404, r#"{"message":"not found"}"#),
    };
    MockResponse::json(200, body)
}
