use super::*;

fn ok(src: &str) -> Value<'_> {
    parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"))
}

fn err(src: &str) -> Error {
    match parse(src) {
        Ok(v) => panic!("{src:?} must fail, got {v:?}"),
        Err(e) => e,
    }
}

#[test]
fn scalars() {
    assert_eq!(ok("null"), Value::Null);
    assert_eq!(ok(" true "), Value::Bool(true));
    assert_eq!(ok("false"), Value::Bool(false));
    assert_eq!(ok("0"), Value::Number(0.0));
    assert_eq!(ok("-12.5"), Value::Number(-12.5));
    assert_eq!(ok("1e3"), Value::Number(1000.0));
    assert_eq!(ok("2.5E-2"), Value::Number(0.025));
    assert_eq!(ok(r#""hi""#), Value::String("hi".into()));
}

#[test]
fn numbers_exact() {
    // Mantissa < 2^53 and exponent within ±22: bit-for-bit like std.
    for s in [
        "0.1", "0.2", "0.3", "123456789012345", "0.000001", "3.141592653589793", "-0.0", "1e3", "2.5E-2",
        "9007199254740991", "1.5e22", "7e-22", "0.00000000000000000001", "12.375", "-480.5",
    ] {
        let v = ok(s).as_f64().unwrap();
        assert_eq!(v.to_bits(), s.parse::<f64>().unwrap().to_bits(), "{s}");
    }
}

#[test]
fn numbers_approximate() {
    // Long mantissas and large exponents: relative error ~1e-15.
    for s in [
        "1.7976931348623157", "1234567890123456789", "9007199254740993", "1e308", "1.7976931348623157e308",
        "2.2250738585072014e-308", "123456789012345678901234567890", "0.1e-30", "6.02214076e23", "1e-100",
    ] {
        let v = ok(s).as_f64().unwrap();
        let want = s.parse::<f64>().unwrap();
        assert!(((v - want) / want).abs() < 1e-15, "{s}: {v} vs {want}");
    }
    assert_eq!(ok("1e400").as_f64(), Some(f64::INFINITY));
    assert_eq!(ok("1e-400").as_f64(), Some(0.0));
    assert_eq!(ok("1e99999999999").as_f64(), Some(f64::INFINITY));
}

#[test]
fn bad_numbers() {
    err("01");
    err("1.");
    err("-");
    err("1e");
    err(".5");
    err("+1");
}

#[test]
fn strings() {
    assert_eq!(ok(r#""a\"b\\c\/d\n""#).as_str(), Some("a\"b\\c/d\n"));
    assert_eq!(ok(r#""Aé""#).as_str(), Some("Aé"));
    assert_eq!(ok(r#""😀""#).as_str(), Some("😀"));
    assert_eq!(ok(r#""Привет""#).as_str(), Some("Привет")); // non-ASCII on purpose: UTF-8 passes through
    // Without escapes: a slice of the input, no copy.
    assert!(matches!(ok(r#""plain""#), Value::String(Cow::Borrowed(_))));
    err(r#""\ud83d""#);
    err(r#""\x""#);
    err("\"a\nb\"");
    err(r#""abc"#);
}

#[test]
fn containers() {
    let v = ok(r#"{"a": [1, 2, {"b": null}], "c": {}}"#);
    assert_eq!(v.get("a").unwrap().as_array().unwrap().len(), 3);
    assert_eq!(v.get("c").unwrap().as_object().unwrap().len(), 0);
    assert_eq!(ok("[]"), Value::Array(vec![]));
    assert_eq!(ok("[[],[]]"), Value::Array(vec![Value::Array(vec![]), Value::Array(vec![])]));
    assert_eq!(ok("[{},{}]").as_array().unwrap().len(), 2);
}

#[test]
fn extensions() {
    let v = ok(r#"
        // line comment
        {
            size: [1, 2,],   /* block
                                comment */
            $id: "x",
            "quoted": true,
        }
    "#);
    assert_eq!(v.get("size").unwrap().as_array().unwrap().len(), 2);
    assert_eq!(v.get("$id").unwrap().as_str(), Some("x"));
    assert_eq!(v.get("quoted").unwrap().as_bool(), Some(true));
}

#[test]
fn structural_errors() {
    err("[1 2]");
    err("[,1]");
    err("{,}");
    err("[1,,2]");
    err("{a 1}");
    err("{a: 1 b: 2}");
    err("{1: 2}");
    err("[1}");
    err("{a: 1]");
    err("[");
    err("[] x");
    err("/* open");
    err("truex");
    err("");
}

#[test]
fn error_position() {
    let e = err("{\n  a: 1,\n  b: ?\n}");
    assert_eq!((e.line, e.col), (3, 6));
    let e = err("[\"ж\", ?]"); // column counts characters, not UTF-8 bytes
    assert_eq!((e.line, e.col), (1, 7));
    assert!(!e.msg.contains("near"), "short line: no fragment");
}

#[test]
fn error_fragment_on_long_line() {
    // JSON on a single line (UISON_JSON in C): a fragment around the error (Cyrillic checks UTF-8 boundaries).
    let src = "{ size: [320, 200], data: { n: 0 }, root: { Column: { gap: 12, children: [ { Text: { text: \"Привет\" ? } } ] } } }";
    let e = err(src);
    assert_eq!(e.line, 1);
    assert!(e.msg.contains("near \"…") && e.msg.contains("text: \"Привет\" <here> ? } } ] } } }\""), "{}", e.msg);
}

#[test]
fn depth_limit() {
    let deep = "[".repeat(200) + &"]".repeat(200);
    assert!(err(&deep).msg.contains("deep"));
    let ok_deep = "[".repeat(100) + &"]".repeat(100);
    ok(&ok_deep);
}

#[test]
fn pull_api() {
    let mut p = Parser::new("{ x: [1, 2], skip: {a: [true, null]}, y: \"s\" }");
    p.begin_object().unwrap();
    let mut seen = vec![];
    while let Some(k) = p.next_key().unwrap() {
        match &*k {
            "x" => {
                p.begin_array().unwrap();
                while p.next_item().unwrap() {
                    seen.push(p.number().unwrap());
                }
            }
            "y" => assert_eq!(p.string().unwrap(), "s"),
            _ => p.skip_value().unwrap(),
        }
    }
    p.finish().unwrap();
    assert_eq!(seen, [1.0, 2.0]);
}

#[test]
fn write_roundtrip() {
    let s = "q\"b\\n\n\t\u{1}ж😀";
    let mut out = String::new();
    write_str(&mut out, s);
    assert_eq!(ok(&out).as_str(), Some(s));
}

#[test]
fn garbage_never_panics() {
    // A simple deterministic "fuzzer": mutations of a valid document.
    let base = r#"{ a: [1, -2.5e3, "xA\n"], b: { c: true, d: null }, // c
        e: "Привет" /* z */ }"#.as_bytes();
    let alphabet = b"{}[],:\"\\/*-+.eE0123456789tfnul \nabc\xD0\xB6";
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..20_000 {
        let mut doc = base.to_vec();
        for _ in 0..(rnd() % 4 + 1) {
            let at = (rnd() as usize) % doc.len();
            match rnd() % 3 {
                0 => {
                    doc.remove(at);
                }
                1 => doc.insert(at, alphabet[(rnd() as usize) % alphabet.len()]),
                _ => doc[at] = alphabet[(rnd() as usize) % alphabet.len()],
            }
        }
        if let Ok(s) = std::str::from_utf8(&doc) {
            let _ = parse(s);
        }
    }
}

#[test]
fn owned_mutation_and_writing() {
    let src = String::from(r#"{ a: 1, b: [true, null, "x\"y"], c: { d: 2.5 } }"#);
    let mut v = parse(&src).unwrap().into_owned();
    drop(src);
    v.set("a", 7);
    v.set("e", "new");
    v.get_mut("c").unwrap().set("d", vec![1, 2]);
    assert_eq!(v.remove("b").unwrap().as_array().unwrap().len(), 3);
    assert_eq!(v.to_json(), r#"{"a":7,"c":{"d":[1,2]},"e":"new"}"#);
    let pretty = v.to_json_pretty();
    assert!(pretty.contains("\n  \"c\": {\n    \"d\": [\n"), "{pretty}");
    assert_eq!(parse(&pretty).unwrap(), v);
    assert!(Value::from(1.5).truthy() && !Value::from(0).truthy() && !Value::from("").truthy());
    assert_eq!(Value::from(Some(3)), Value::Number(3.0));
    assert_eq!(Value::from(None::<i32>), Value::Null);
}

#[test]
fn compact_to_strict_one_line() {
    use crate::compact;
    let src = "{size:[400, 300],background:\"#F5F5F7\", // a comment\n items:[\n{rect:[1,2.5,-3e2,4],fill:\"#fff\"},\n/* block */ {text:\"a: b, // c \\\" d\",at:[0,0],bold:true},\n{hit:[0,0,1,1],id:\"x\",cursor:null},\n]}\n";
    let out = compact(src);
    assert_eq!(
        out,
        r##"{"size":[400,300],"background":"#F5F5F7","items":[{"rect":[1,2.5,-3e2,4],"fill":"#fff"},{"text":"a: b, // c \" d","at":[0,0],"bold":true},{"hit":[0,0,1,1],"id":"x","cursor":null}]}"##
    );
    // The result is strict JSON with the same value.
    assert_eq!(crate::parse(&out).unwrap(), crate::parse(src).unwrap());
    assert!(!out.contains('\n'));
    // Already strict JSON stays the same; unicode passes through.
    let strict = r#"{"a":[1,{"b":"ж😀"}],"c":false}"#;
    assert_eq!(compact(strict), strict);
    assert_eq!(compact("[1,2,]"), "[1,2]");
    assert_eq!(compact("{ k : 1 , }"), r#"{"k":1}"#);
}
