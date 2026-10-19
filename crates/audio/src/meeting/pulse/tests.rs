use super::*;
use crate::meeting::tests::Directory;
use std::io::Read;
use std::os::unix::net::UnixListener;

#[test]
fn explicit_endpoints_never_fall_back_to_another_or_remote_server() {
    for invalid in ["tcp:localhost", "/tmp/native", "unix:relative", "", "unix:"] {
        assert!(explicit_socket(invalid.as_ref()).is_err());
    }
    assert_eq!(
        explicit_socket("unix:/tmp/private/native".as_ref()).expect("local socket"),
        std::path::PathBuf::from("/tmp/private/native")
    );
}

#[test]
fn an_unresponsive_authentication_handshake_has_an_absolute_deadline() {
    let dir = Directory::new();
    std::fs::create_dir(&dir.0).expect("directory");
    let path = dir.0.join("native");
    let listener = UnixListener::bind(&path).expect("local fixture");
    let peer = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("client");
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("fixture timeout");
        let mut bytes = Vec::new();
        socket
            .read_to_end(&mut bytes)
            .expect("client closes without our reply");
    });
    let started = Instant::now();
    assert!(Connection::connect(&path, None, 0, None).is_err());
    assert!(started.elapsed() < Duration::from_secs(5));
    peer.join().expect("fixture stopped");
}

#[test]
fn cancellation_breaks_a_handshake_without_a_server_reply() {
    let dir = Directory::new();
    std::fs::create_dir(&dir.0).expect("directory");
    let path = dir.0.join("native");
    let listener = UnixListener::bind(&path).expect("local fixture");
    let control = Control::new();
    let shared = control.clone();
    let worker = std::thread::spawn(move || Connection::connect(&path, Some(shared), 60, None));
    let (mut socket, _) = listener.accept().expect("client connected");
    let started = Instant::now();
    control.request(true);
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("fixture timeout");
    let mut bytes = Vec::new();
    socket
        .read_to_end(&mut bytes)
        .expect("client closes without our reply");
    assert!(worker.join().expect("client worker").is_err());
    assert!(started.elapsed() < Duration::from_secs(1));
}
