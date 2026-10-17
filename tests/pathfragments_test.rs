use aiagents::pathfragments::expand_path_fragments;

#[test]
fn expands_fragments_in_complex_paths() {
    let home = std::env::temp_dir().canonicalize().unwrap();
    let project = home.join(format!("aiagents-pathfragment-test-{}", std::process::id()));
    std::fs::create_dir_all(&project).unwrap();
    let rel = project.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(
        expand_path_fragments("{{relprojectdir}}", &project, &home).unwrap(),
        rel
    );
    assert_eq!(
        expand_path_fragments("{{home}}/.project/{{relprojectdir}}", &project, &home).unwrap(),
        format!("{}/.project/{}", home.display(), rel)
    );
    assert_eq!(
        expand_path_fragments("{{projectdir}}", &project, &home).unwrap(),
        project.to_string_lossy()
    );
    assert!(expand_path_fragments("{{invalid}}", &project, &home).is_err());
    std::fs::remove_dir(&project).unwrap();
}
