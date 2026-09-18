use rawk::test_support as harness;
use std::{path::Path, process::Command, time::Duration};

fn compare(program: &str, input: &[u8]) {
    let run = |bin: &Path| {
        let mut cmd = Command::new(bin);
        cmd.arg(program);
        harness::run(cmd, input, Duration::from_secs(3)).unwrap()
    };
    let expected = run(&harness::reference_binary().unwrap());
    assert!(
        !expected.timed_out && expected.code == Some(0),
        "invalid oracle case: {program}: {expected:?}"
    );
    let actual = run(Path::new(env!("CARGO_BIN_EXE_rawk")));
    assert_eq!(actual, expected, "{program}, input={input:?}");
}

#[test]
fn arrays_forwarding_recursion_and_local_parameters() {
    for program in [
        "function f(a){a[1]=9} function g(b){f(b)} BEGIN{x[1]=1;g(x);print x[1]}",
        "function f(a,n){if(n)f(a,n-1);a[n]=n} BEGIN{f(x,3);print x[0],x[3]}",
        "function f(a){a[1]=9;print a[1]} BEGIN{a[1]=1;f();print a[1]}",
        "function f(a){split(\"a:b\",a,\":\")} BEGIN{f(x);print x[1],x[2]}",
        "function f(x){x++;return x} BEGIN{x=3;print f(x),x}",
    ] {
        compare(program, b"");
    }
}

#[test]
fn regex_compositions_match_original() {
    for pattern in [
        "a|ab",
        "ab|a",
        "(a|ab)*",
        "a*",
        "a?",
        "^a|b$",
        "[[:digit:]]+",
        "[^a]+",
        "",
        "a.*b|a",
    ] {
        for subject in ["", "ab", "aabb", "ba1b"] {
            compare(
                &format!(
                    "BEGIN{{s=\"{subject}\";match(s,/{pattern}/);print RSTART,RLENGTH; print gsub(/{pattern}/,\"X\",s),s}}"
                ),
                b"",
            );
        }
    }
}

#[test]
fn composed_input_and_control_flow() {
    for (program, input) in [
        (
            "BEGIN{getline x;print x,NR} {print $0,NR}",
            b"a\nb\n".as_slice(),
        ),
        ("{getline; print $0,NR,FNR}", b"a\nb\nc\nd\n".as_slice()),
        (
            "BEGIN{RS=\"\";FS=\",\"} {print NF,$1,$2,$3,$4}",
            b"a,b\nc,d\n\n".as_slice(),
        ),
        ("BEGIN{RS=\"a|ab\"} {print $0}", b"xabzab".as_slice()),
        ("BEGIN{exit 7} END{print \"END\";exit 0}", b"".as_slice()),
    ] {
        compare(program, input);
    }
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 64,
        rng_seed: proptest::test_runner::RngSeed::Fixed(20260919),
        ..proptest::test_runner::Config::default()
    })]
    #[test]
    fn generated_record_array_function_programs(
        values in proptest::collection::vec(-100i32..100, 0..20),
        increment in -5i32..6,
        buckets in 1usize..6,
    ) {
        let program = format!(
            "function add(a,k,v) {{a[k]+=v; return a[k]}} \
             {{ k=NR%{buckets}; x=$1; y=x++; add(a,k,y+({increment})); s+=x }} \
             END {{for(i=0;i<{buckets};i++) print i,a[i]+0; print NR,s+0}}"
        );
        let input = values.iter().map(|n| format!("{n}\n")).collect::<String>();
        compare(&program, input.as_bytes());
    }
}
