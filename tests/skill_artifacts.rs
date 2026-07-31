use sha2::{Digest, Sha256};

#[test]
fn skill_file_has_the_expected_byte_hash() {
    let bytes = include_bytes!("../skills/okf-wiki/SKILL.md");

    let digest = Sha256::digest(bytes);

    assert_eq!(
        format!("{digest:x}"),
        "857dbad2d3fbfa5ffbccf3cd7b6e75565c12aa0c6ae465ea225a3e6eab2afb6f"
    );
}
