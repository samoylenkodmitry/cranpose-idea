use super::*;

#[test]
fn only_empty_owned_dioxus_inputs_are_omitted() -> Result<()> {
    let root = tempfile::tempdir()?;
    let cache = root.path().join("target");
    fs::create_dir(&cache)?;
    let empty = cache.join("libdeps-1234abcd.a");
    fs::write(&empty, [])?;
    let arguments: Vec<OsString> = vec![
        "-m64".into(),
        "-Wl,--whole-archive".into(),
        empty.as_os_str().into(),
        "-Wl,--no-whole-archive".into(),
        "dependency.rlib".into(),
        "-o".into(),
        "app".into(),
    ];
    let filtered = filter_empty_archives(arguments.clone(), &cache);
    assert_eq!(filtered.len(), arguments.len() - 1);
    assert_eq!(&filtered[2..], &arguments[3..]);
    assert!(empty.exists(), "the shared cache is never modified");
    assert_eq!(fs::metadata(&empty)?.len(), 0);

    fs::write(&empty, b"!<arch>\n")?;
    assert_eq!(filter_empty_archives(arguments.clone(), &cache), arguments);
    fs::write(&empty, [])?;
    assert_eq!(
        filter_empty_archives(arguments.clone(), root.path().join("other").as_path()),
        arguments
    );
    assert!(!empty_owned_archive(&cache.join("missing.a"), &cache));

    let ordinary = cache.join("user.a");
    fs::write(&ordinary, [])?;
    assert!(!empty_owned_archive(&ordinary, &cache));
    let outside = root.path().join("libdeps-8765abcd.a");
    fs::write(&outside, [])?;
    assert!(!empty_owned_archive(&outside, &cache));
    fs::remove_file(&empty)?;
    std::os::unix::fs::symlink(&outside, &empty)?;
    assert!(!empty_owned_archive(&empty, &cache));
    Ok(())
}
