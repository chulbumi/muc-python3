//! 한글 처리 모듈
//!
//! 완성형 한글 확인 및 한글 조사(이/가, 을/를, 은/는 등) 선택 기능을 제공합니다.

/// 한글 유니코드 범위
const HANGUL_SYLLABLE_START: char = '\u{AC00}';
const HANGUL_SYLLABLE_END: char = '\u{D7A3}';

/// 한글 종성 개수
const JONGUNG_COUNT: u32 = 28;

/// ㄹ 받침의 종성 인덱스 (8번째)
const JONGUNG_RIEUL: u32 = 8;

/// 완성형 한글 여부를 확인합니다.
///
/// # Arguments
///
/// * `word` - 확인할 문자열
///
/// # Returns
///
/// 모든 문자가 완성형 한글(U+AC00 ~ U+D7A3)이면 true, 아니면 false
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::is_han;
///
/// assert!(is_han("한글"));
/// assert!(is_han("안녕"));
/// assert!(!is_han("ABC"));
/// assert!(!is_han("한글ABC"));
/// assert!(!is_han(""));
/// ```
pub fn is_han(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }

    word.chars()
        .all(|c| c >= HANGUL_SYLLABLE_START && c <= HANGUL_SYLLABLE_END)
}

/// 한글 문자의 받침(종성)이 있는지 확인합니다.
///
/// # Arguments
///
/// * `c` - 확인할 한글 문자
///
/// # Returns
///
/// 받침이 있으면 true, 없으면 false
///
/// # Notes
///
/// 완성형 한글은 초성(21개) × 중성(28개) × 종성(28개)로 구성됩니다.
/// 종성 인덱스가 0이면 받침이 없고, 1~27이면 받침이 있습니다.
fn has_jongung(c: char) -> bool {
    if c < HANGUL_SYLLABLE_START || c > HANGUL_SYLLABLE_END {
        return false;
    }

    let code = c as u32 - HANGUL_SYLLABLE_START as u32;
    let jongung_index = code % JONGUNG_COUNT;

    jongung_index > 0
}

/// 한글 문자의 받침이 ㄹ인지 확인합니다.
///
/// # Arguments
///
/// * `c` - 확인할 한글 문자
///
/// # Returns
///
/// 받침이 ㄹ(8번째 종성)이면 true, 아니면 false
///
/// # Notes
///
/// ㄹ 받침은 특별한 예외 처리가 필요합니다:
/// - (으)로 → 로
/// - 과(와) → 와
fn has_rieul_jongung(c: char) -> bool {
    if c < HANGUL_SYLLABLE_START || c > HANGUL_SYLLABLE_END {
        return false;
    }

    let code = c as u32 - HANGUL_SYLLABLE_START as u32;
    let jongung_index = code % JONGUNG_COUNT;

    jongung_index == JONGUNG_RIEUL
}

/// 단어의 마지막 문자가 받침이 있는지 확인합니다.
///
/// # Arguments
///
/// * `word` - 확인할 단어
///
/// # Returns
///
/// 마지막 문자가 한글이고 받침이 있으면 true, 아니면 false
fn has_final_consonant(word: &str) -> bool {
    word.chars().last().is_some_and(has_jongung)
}

/// 단어의 마지막 문자가 ㄹ 받침인지 확인합니다.
///
/// # Arguments
///
/// * `word` - 확인할 단어
///
/// # Returns
///
/// 마지막 문자가 한글이고 ㄹ 받침이면 true, 아니면 false
fn has_rieul_final(word: &str) -> bool {
    word.chars().last().is_some_and(has_rieul_jongung)
}

/// 이/가 조사를 선택합니다.
///
/// 받침이 있으면 "이", 없으면 "가"를 반환합니다.
///
/// # Arguments
///
/// * `word` - 조사를 선택할 단어
///
/// # Returns
///
/// 선택된 조사 ("이" 또는 "가")
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::han_iga;
///
/// assert_eq!(han_iga("철"), "이");    // ㄹ 받침
/// assert_eq!(han_iga("민지"), "가"); // 받침 없음
/// assert_eq!(han_iga("사과"), "가"); // 받침 없음
/// ```
pub fn han_iga(word: &str) -> &'static str {
    if has_final_consonant(word) {
        "이"
    } else {
        "가"
    }
}

/// 이라 조사를 선택합니다.
///
/// 받침이 있으면 "이라", 없으면 "라"를 반환합니다.
///
/// # Arguments
///
/// * `word` - 조사를 선택할 단어
///
/// # Returns
///
/// 선택된 조사 ("이라" 또는 "라")
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::han_ira;
///
/// assert_eq!(han_ira("철"), "이라");   // ㄹ 받침
/// assert_eq!(han_ira("민지"), "라");  // 받침 없음
/// ```
pub fn han_ira(word: &str) -> &'static str {
    if has_final_consonant(word) {
        "이라"
    } else {
        "라"
    }
}

/// 을/를 조사를 선택합니다.
///
/// 받침이 있으면 "을", 없으면 "를"을 반환합니다.
///
/// # Arguments
///
/// * `word` - 조사를 선택할 단어
///
/// # Returns
///
/// 선택된 조사 ("을" 또는 "를")
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::han_obj;
///
/// assert_eq!(han_obj("철"), "을");    // ㄹ 받침
/// assert_eq!(han_obj("민지"), "를");  // 받침 없음
/// ```
pub fn han_obj(word: &str) -> &'static str {
    if has_final_consonant(word) {
        "을"
    } else {
        "를"
    }
}

/// 은/는 조사를 선택합니다.
///
/// 받침이 있으면 "은", 없으면 "는"을 반환합니다.
///
/// # Arguments
///
/// * `word` - 조사를 선택할 단어
///
/// # Returns
///
/// 선택된 조사 ("은" 또는 "는")
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::han_un;
///
/// assert_eq!(han_un("철"), "은");    // ㄹ 받침
/// assert_eq!(han_un("민지"), "는");  // 받침 없음
/// ```
pub fn han_un(word: &str) -> &'static str {
    if has_final_consonant(word) {
        "은"
    } else {
        "는"
    }
}

/// 과/와 조사를 선택합니다.
///
/// 받침이 있으면 "과", 없으면 "와"를 반환합니다.
/// 단, ㄹ 받침의 경우 "와"를 반환합니다.
///
/// # Arguments
///
/// * `word` - 조사를 선택할 단어
///
/// # Returns
///
/// 선택된 조사 ("과" 또는 "와")
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::han_wa;
///
/// assert_eq!(han_wa("밥"), "과");    // ㅂ 받침
/// assert_eq!(han_wa("민지"), "와");  // 받침 없음
/// assert_eq!(han_wa("날"), "와");    // ㄹ 받침 예외
/// ```
pub fn han_wa(word: &str) -> &'static str {
    if has_final_consonant(word) && !has_rieul_final(word) {
        "과"
    } else {
        "와"
    }
}

/// (으)로 조사를 선택합니다.
///
/// 받침이 있으면 "으로", 없으면 "로"를 반환합니다.
/// 단, ㄹ 받침의 경우 "로"를 반환합니다.
///
/// # Arguments
///
/// * `word` - 조사를 선택할 단어
///
/// # Returns
///
/// 선택된 조사 ("으로" 또는 "로")
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::han_uro;
///
/// assert_eq!(han_uro("밥"), "으로");  // ㅂ 받침
/// assert_eq!(han_uro("민지"), "로");  // 받침 없음
/// assert_eq!(han_uro("날"), "로");    // ㄹ 받침 예외
/// ```
pub fn han_uro(word: &str) -> &'static str {
    if has_final_consonant(word) && !has_rieul_final(word) {
        "으로"
    } else {
        "로"
    }
}

/// 이(Empty) 조사를 선택합니다.
///
/// 받침이 있으면 "이", 없으면 빈 문자열을 반환합니다.
///
/// # Arguments
///
/// * `word` - 조사를 선택할 단어
///
/// # Returns
///
/// 선택된 조사 ("이" 또는 "")
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::han_i;
///
/// assert_eq!(han_i("철"), "이");    // ㄹ 받침
/// assert_eq!(han_i("민지"), "");    // 받침 없음
/// ```
pub fn han_i(word: &str) -> &'static str {
    if has_final_consonant(word) {
        "이"
    } else {
        ""
    }
}

/// 야 조사를 선택합니다.
///
/// 받침이 있으면 "아야", 없으면 "야"를 반환합니다.
///
/// # Arguments
///
/// * `word` - 조사를 선택할 단어
///
/// # Returns
///
/// 선택된 조사 ("아야" 또는 "야")
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::han_aya;
///
/// assert_eq!(han_aya("밥"), "아야");  // ㅂ 받침
/// assert_eq!(han_aya("민지"), "야");  // 받침 없음
/// ```
pub fn han_aya(word: &str) -> &'static str {
    if has_final_consonant(word) {
        "아야"
    } else {
        "야"
    }
}

/// ANSI 이스케이프 시퀀스(\x1b...[^m]*m)를 제거. 파이썬 lib/func.stripANSI.
/// UTF-8 문자 단위로 복사(바이트 as char 사용 금지: 한글이 깨져 조사 선택 오류).
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == 27 {
            // skip until 'm'
            i += 1;
            while i < bytes.len() && bytes[i] != b'm' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        if bytes[i] == 155 {
            i += 1;
            continue;
        }
        if bytes[i] == 8 {
            out.pop();
            i += 1;
            continue;
        }
        // UTF-8 문자 단위로 복사 (바이트 as char는 한글 등 멀티바이트 깨짐 → 조사 '가' 오류)
        if let Some(c) = s[i..].chars().next() {
            out.push(c);
            i += c.len_utf8();
        } else {
            break;
        }
    }
    out
}

/// (이/가), (을/를) 등 조사 패턴을 한 번 치환. 괄호 앞 문자열(ANSI 제거)의 받침에 따라 선택.
/// 파이썬 lib/hangul.postPosition1. 한 번에 하나만 치환하고 반환.
pub fn post_position1(line: &str) -> String {
    let s = match line.find('(') {
        Some(i) => i,
        None => return line.to_string(),
    };
    let e = match line.find(')') {
        Some(i) => i,
        None => return line.to_string(),
    };
    if e <= s {
        return line.to_string();
    }
    let pps = &line[s..=e];
    let word = strip_ansi(&line[..s]).trim_end().to_string();

    // (으)로: / 가 없어서 별도 처리. "(으)로" 전체를 han_uro 결과로 치환.
    if pps == "(으)" {
        if let Some(rest) = line.get(e + 1..) {
            if rest.starts_with("로") {
                let picked = han_uro(&word);
                let end = e + 1 + "로".len();
                let mut out = String::with_capacity(line.len());
                out.push_str(&line[..s]);
                out.push_str(picked);
                out.push_str(&line[end..]);
                return out;
            }
        }
    }

    let sep = match pps.find('/') {
        Some(i) => i,
        None => return line.to_string(),
    };
    let form = pps[1..sep].trim();
    let picked: &str = match form {
        "이" if pps.contains("/가") => han_iga(&word),
        "을" if pps.contains("/를") => han_obj(&word),
        "에게" => "에게",
        "와" if pps.contains("/과") => han_wa(&word),
        "의" => "의",
        "은" if pps.contains("/는") => han_un(&word),
        "이라" if pps.contains("/라") => han_ira(&word),
        _ => form,
    };
    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..s]);
    out.push_str(picked);
    out.push_str(&line[e + 1..]);
    out
}

/// post_position1을 (x/y) 패턴이 없을 때까지 반복 적용. 감정표현 makeScript용.
pub fn post_position_all(line: &str) -> String {
    let mut cur = line.to_string();
    loop {
        let next = post_position1(&cur);
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;

    // is_han 테스트
    #[test]
    fn test_is_han_valid_korean() {
        assert!(is_han("한글"));
        assert!(is_han("안녕"));
        assert!(is_han("철수"));
        assert!(is_han("민지"));
        assert!(is_han("사과"));
        assert!(is_han("가"));
    }

    #[test]
    fn test_is_han_invalid() {
        assert!(!is_han(""));
        assert!(!is_han("ABC"));
        assert!(!is_han("한글ABC"));
        assert!(!is_han("123"));
        assert!(!is_han("Hello"));
        assert!(!is_han("한글123"));
        assert!(!is_han("a"));
    }

    // 받침 관련 내부 함수 테스트
    #[test]
    fn test_has_final_consonant() {
        // 받침 있는 경우 (글 = ㄹ 받침)
        assert!(has_final_consonant("한글")); // 글: ㄹ 받침
        assert!(has_final_consonant("밥")); // 밥: ㅂ 받침
        assert!(has_final_consonant("철")); // 철: ㄹ 받침
        assert!(has_final_consonant("값")); // 값: ㄱㅅ 받침
        assert!(has_final_consonant("앉")); // 앉: ㄴㅈ 받침

        // 받침 없는 경우
        assert!(!has_final_consonant("가")); // 가: 받침 없음
        assert!(!has_final_consonant("민지")); // 지: 받침 없음
        assert!(!has_final_consonant("나")); // 나: 받침 없음
        assert!(!has_final_consonant("사과")); // 과: 받침 없음
        assert!(!has_final_consonant("수")); // 수: 받침 없음
    }

    #[test]
    fn test_has_rieul_final() {
        // ㄹ 받침 있는 경우
        assert!(has_rieul_final("날")); // 날: ㄹ 받침
        assert!(has_rieul_final("물")); // 물: ㄹ 받침
        assert!(has_rieul_final("불")); // 불: ㄹ 받침
        assert!(has_rieul_final("살")); // 살: ㄹ 받침
        assert!(has_rieul_final("철")); // 철: ㄹ 받침
        assert!(has_rieul_final("한글")); // 글: ㄹ 받침

        // ㄹ 받침 없는 경우
        assert!(!has_rieul_final("가")); // 받침 없음
        assert!(!has_rieul_final("밥")); // ㅂ 받침
        assert!(!has_rieul_final("민지")); // 받침 없음
    }

    // han_iga 테스트
    #[test]
    fn test_han_iga() {
        // 받침 있으면 "이"
        assert_eq!(han_iga("철"), "이"); // ㄹ 받침
        assert_eq!(han_iga("한글"), "이"); // ㄹ 받침
        assert_eq!(han_iga("밥"), "이"); // ㅂ 받침
        assert_eq!(han_iga("당신"), "이"); // 신: ㄴ 받침

        // 받침 없으면 "가"
        assert_eq!(han_iga("민지"), "가"); // 받침 없음
        assert_eq!(han_iga("가"), "가"); // 받침 없음
        assert_eq!(han_iga("나"), "가"); // 받침 없음
        assert_eq!(han_iga("사과"), "가"); // 받침 없음
        assert_eq!(han_iga("수"), "가"); // 받침 없음
    }

    #[test]
    fn test_post_position_dansin_iga() {
        // 당신(받침 ㄴ) + (이/가) → 당신이
        let r = post_position_all("당신(이/가) 웃습니다.");
        assert!(r.starts_with("당신이"), "expected '당신이', got: {:?}", r);
    }

    // han_ira 테스트
    #[test]
    fn test_han_ira() {
        // 받침 있으면 "이라"
        assert_eq!(han_ira("철"), "이라"); // ㄹ 받침
        assert_eq!(han_ira("한글"), "이라"); // ㄹ 받침

        // 받침 없으면 "라"
        assert_eq!(han_ira("민지"), "라"); // 받침 없음
        assert_eq!(han_ira("가"), "라"); // 받침 없음
    }

    // han_obj 테스트
    #[test]
    fn test_han_obj() {
        // 받침 있으면 "을"
        assert_eq!(han_obj("철"), "을"); // ㄹ 받침
        assert_eq!(han_obj("한글"), "을"); // ㄹ 받침

        // 받침 없으면 "를"
        assert_eq!(han_obj("민지"), "를"); // 받침 없음
        assert_eq!(han_obj("가"), "를"); // 받침 없음
    }

    // han_un 테스트
    #[test]
    fn test_han_un() {
        // 받침 있으면 "은"
        assert_eq!(han_un("철"), "은"); // ㄹ 받침
        assert_eq!(han_un("한글"), "은"); // ㄹ 받침

        // 받침 없으면 "는"
        assert_eq!(han_un("민지"), "는"); // 받침 없음
        assert_eq!(han_un("가"), "는"); // 받침 없음
    }

    // han_wa 테스트
    #[test]
    fn test_han_wa() {
        // 받침 있으면 "과"
        assert_eq!(han_wa("밥"), "과"); // ㅂ 받침

        // 받침 없으면 "와"
        assert_eq!(han_wa("민지"), "와"); // 받침 없음
        assert_eq!(han_wa("가"), "와"); // 받침 없음

        // ㄹ 받침 예외: "와"
        assert_eq!(han_wa("날"), "와"); // ㄹ 받침 예외
        assert_eq!(han_wa("물"), "와"); // ㄹ 받침 예외
        assert_eq!(han_wa("철"), "와"); // ㄹ 받침 예외
        assert_eq!(han_wa("한글"), "와"); // ㄹ 받침 예외
    }

    // han_uro 테스트
    #[test]
    fn test_han_uro() {
        // 받침 있으면 "으로"
        assert_eq!(han_uro("밥"), "으로"); // ㅂ 받침

        // 받침 없으면 "로"
        assert_eq!(han_uro("민지"), "로"); // 받침 없음
        assert_eq!(han_uro("가"), "로"); // 받침 없음

        // ㄹ 받침 예외: "로"
        assert_eq!(han_uro("날"), "로"); // ㄹ 받침 예외
        assert_eq!(han_uro("물"), "로"); // ㄹ 받침 예외
        assert_eq!(han_uro("불"), "로"); // ㄹ 받침 예외
        assert_eq!(han_uro("철"), "로"); // ㄹ 받침 예외
    }

    // han_i 테스트
    #[test]
    fn test_han_i() {
        // 받침 있으면 "이"
        assert_eq!(han_i("철"), "이"); // ㄹ 받침
        assert_eq!(han_i("한글"), "이"); // ㄹ 받침

        // 받침 없으면 빈 문자열
        assert_eq!(han_i("민지"), ""); // 받침 없음
        assert_eq!(han_i("가"), ""); // 받침 없음
    }

    // han_aya 테스트
    #[test]
    fn test_han_aya() {
        // 받침 있으면 "아야"
        assert_eq!(han_aya("밥"), "아야"); // ㅂ 받침

        // 받침 없으면 "야"
        assert_eq!(han_aya("민지"), "야"); // 받침 없음
        assert_eq!(han_aya("가"), "야"); // 받침 없음
    }

    // 종합 테스트: 다양한 받침 패턴
    #[test]
    fn test_various_final_consonants() {
        // 겹받침 포함 다양한 받침 테스트
        assert_eq!(han_iga("값"), "이"); // ㄱㅅ 받침
        assert_eq!(han_iga("앉"), "이"); // ㄴㅈ 받침
        assert_eq!(han_iga("닭"), "이"); // ㄹㄱ 받침
        assert_eq!(han_iga("삶"), "이"); // ㄹㅁ 받침

        // ㄹ 받침은 특별 처리
        assert_eq!(han_wa("닭"), "과"); // ㄹㄱ 받침 (ㄹ로 시작하지만 ㄹ이 아님)
        assert_eq!(han_uro("삶"), "으로"); // ㄹㅁ 받침 (ㄹ로 시작하지만 ㄹ이 아님)
        assert_eq!(han_wa("불"), "와"); // ㄹ 받침 예외
        assert_eq!(han_uro("불"), "로"); // ㄹ 받침 예외
    }

    #[test]
    fn test_empty_string() {
        assert!(!is_han(""));
        assert_eq!(han_iga(""), "가"); // 빈 문자열은 받침 없음 처리
        assert_eq!(han_obj(""), "를");
        assert_eq!(han_un(""), "는");
    }
}

/// 한글 단어가 받침(종성)으로 끝나는지 확인합니다.
///
/// # Arguments
///
/// * `word` - 확인할 한글 단어
///
/// # Returns
///
/// 받침(종성)으로 끝나면 true, 없으면 false
///
/// # Examples
///
/// ```
/// use muc_engine::hangul::ends_with_consonant;
///
/// assert!(ends_with_consonant("책")); // 책 ends with ㄱ (has 받침)
/// assert!(ends_with_consonant("책상")); // 상 ends with ㅇ (has 받침)
/// assert!(!ends_with_consonant(""));  // empty string
/// assert!(!ends_with_consonant("가")); // 가 has no 받침
/// ```
pub fn ends_with_consonant(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }

    // Get the last character
    if let Some(last_char) = word.chars().last() {
        // Check if it's a Korean syllable
        if last_char >= HANGUL_SYLLABLE_START && last_char <= HANGUL_SYLLABLE_END {
            // Calculate if it has 받침
            // Unicode formula: (syllable - AC00) % 28
            let offset = last_char as u32 - HANGUL_SYLLABLE_START as u32;
            let jongung_index = offset % 28; // 종성 인덱스 (0 = no 받침, 1-27 = has 받침)

            jongung_index > 0 // 0 means no 받침, 1-27 means has 받침
        } else {
            false
        }
    } else {
        false
    }
}
