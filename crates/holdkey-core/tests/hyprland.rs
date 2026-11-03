use holdkey_core::Action;
use holdkey_core::hyprland::{Drift, check_drift, parse_shortcuts, request};
use std::{io::{Read, Write}, os::unix::net::UnixListener};

const SAMPLE: &str = "com.anthropic.Claude:384B1B3B-Ctrl+Alt+Space -> Claude shortcut: Ctrl+Alt+Space\nhex:dictation -> Dictation\nomarchy:menu.root -> Toggle the root menu\n";

fn act(id: &str) -> Action {
    Action { id: id.into(), description: String::new(), preferred_trigger: "F13".into() }
}

#[test]
fn parses_real_output_shape() {
    let l = parse_shortcuts(SAMPLE);
    assert_eq!(l.len(), 3);
    assert_eq!((l[1].app_id.as_str(), l[1].id.as_str(), l[1].description.as_str()), ("hex", "dictation", "Dictation"));
    assert_eq!(l[0].app_id, "com.anthropic.Claude");
}

#[test]
fn flags_wrong_app_id_and_missing() {
    let l = parse_shortcuts(SAMPLE);
    assert!(check_drift(&l, "hex", &[act("dictation")]).is_empty());
    assert_eq!(
        check_drift(&l, "dev.holdkey.wrong", &[act("dictation"), act("nope")]),
        vec![
            Drift::WrongAppId { action: "dictation".into(), found: "hex".into() },
            Drift::Missing { action: "nope".into() },
        ]
    );
}

// The server reads to EOF after sending, which only returns 0 if the client dropped its stream.
#[test]
fn request_writes_command_and_closes_connection() {
    let path = std::env::temp_dir().join(format!("hl-test-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).unwrap();
    let server = std::thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        let mut buf = [0u8; 64];
        let n = s.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"globalshortcuts");
        s.write_all(SAMPLE.as_bytes()).unwrap();
        s.shutdown(std::net::Shutdown::Write).unwrap();
        assert_eq!(s.read(&mut buf).unwrap(), 0, "client must close its end");
    });
    assert_eq!(request(&path, "globalshortcuts").unwrap(), SAMPLE);
    server.join().unwrap();
    std::fs::remove_file(path).unwrap();
}
