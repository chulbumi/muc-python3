use muc_engine::hangul::is_han;

fn main() {
    let test_names = vec![
        "테스터러스트",
        "테스터",
        "철수",
        "민지",
        "한글",
        "손님",
    ];

    for name in test_names {
        println!("is_han('{}') = {}", name, is_han(name));
    }
}
