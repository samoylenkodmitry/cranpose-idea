use super::*;

#[test]
fn covers_every_bundled_platform_and_rejects_unknown_hosts() {
    for os in ["macos", "linux", "windows"] {
        for arch in ["aarch64", "x86_64"] {
            let (_, digest) = distribution(os, arch).expect("supported host");
            assert_eq!(digest.len(), 64);
            assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
    }
    assert!(distribution("linux", "riscv64").is_err());
}

#[test]
fn refuses_a_modified_download() {
    let bytes = b"verified archive";
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert!(verify_archive(bytes, &digest).is_ok());
    assert!(verify_archive(b"modified archive", &digest).is_err());
}
