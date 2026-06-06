use aspen_uci::{UciOptionKind, UciOptions};

#[derive(Default, UciOptions)]
struct AllKinds {
    #[uci(name = "Hash", min = 1, max = 1024, default = 16)]
    hash: i64,
    #[uci(name = "Ponder", default = true)]
    ponder: bool,
    #[uci(name = "Style", combo, default = "Solid", variants = ["Solid", "Aggressive"])]
    style: String,
    #[uci(name = "SyzygyPath")]
    syzygy_path: String,
    #[uci(name = "Clear Hash", button)]
    #[allow(dead_code)]
    clear_hash: (),
}

#[test]
fn declarations_and_set_cover_all_kinds() {
    let declarations = AllKinds::declarations();
    assert_eq!(declarations.len(), 5);

    let rendered: Vec<String> = declarations.iter().map(|line| line.to_string()).collect();
    assert!(rendered.iter().any(|line| {
        line == "option name Hash type spin default 16 min 1 max 1024"
    }));
    assert!(rendered.iter().any(|line| line == "option name Ponder type check default true"));
    assert!(rendered.iter().any(|line| {
        line == "option name Style type combo default Solid var Solid var Aggressive"
    }));
    assert!(rendered.iter().any(|line| line == "option name SyzygyPath type string default "));
    assert!(rendered.iter().any(|line| line == "option name Clear Hash type button"));
    assert!(matches!(
        declarations[0].kind,
        UciOptionKind::Spin { default: 16, min: 1, max: 1024 }
    ));

    let mut options = AllKinds::default();
    options.set("Hash", Some("256")).unwrap();
    assert_eq!(options.hash, 256);
    options.set("Ponder", Some("false")).unwrap();
    assert!(!options.ponder);
    options.set("Style", Some("Aggressive")).unwrap();
    assert_eq!(options.style, "Aggressive");
    options.set("SyzygyPath", Some("/tables")).unwrap();
    assert_eq!(options.syzygy_path, "/tables");
    options.set("Clear Hash", None).unwrap();
    assert!(options.set("Nonexistent", Some("x")).is_err());
}
