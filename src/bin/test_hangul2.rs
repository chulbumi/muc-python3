use muc_engine::hangul::is_han;

fn main() {
    // Test various edge cases that might be received from telnet
    let test_cases = vec![
        ("테스터러스트", true),
        ("테스터", true),
        ("테스터 ", false),   // trailing space
        (" 테스터", false),   // leading space
        ("테스터\r", false),  // with CR
        ("테스터\n", false),  // with LF
        ("테스터\0", false),  // with null byte
        ("", false),          // empty
        ("ABC", false),       // non-Korean
        ("테스터ABC", false), // mixed
    ];

    println!("Testing is_han function:");
    println!("{:<20} | expected | actual | match?", "=".repeat(60));
    println!(
        "{:<20} | {:>8} | {:>6} | result",
        "input", "expected", "actual"
    );

    for (input, expected) in test_cases {
        let result = is_han(input);
        let match_result = if result == expected { "OK" } else { "FAIL" };
        println!(
            "{:<20} | {:>8} | {:>6} | {}",
            format!("{:?}", input),
            expected,
            result,
            match_result
        );
    }
}
