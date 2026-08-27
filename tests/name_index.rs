use cfb::CompoundFile;
use std::io::{Cursor, Read, Write};

/// Every name a walk reports resolves through the path lookup, in its own
/// spelling and in another case, and a name that was removed does not —
/// on the file being written and on it reopened, with a directory large
/// enough that the sibling tree is deep.
#[test]
fn every_walked_name_resolves_and_removed_names_do_not() {
    let mut comp = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    for i in 0..600 {
        let storage = format!("/Storage{i}");
        comp.create_storage(&storage).unwrap();
        for name in ["Data", "Parameters", "Wide"] {
            let mut stream =
                comp.create_stream(format!("{storage}/{name}")).unwrap();
            stream.write_all(&[i as u8; 40]).unwrap();
        }
    }
    for i in (0..600).step_by(4) {
        comp.remove_stream(format!("/Storage{i}/Wide")).unwrap();
    }
    for i in (0..600).step_by(8) {
        comp.remove_storage_all(format!("/Storage{i}")).unwrap();
        comp.create_storage(format!("/storage{i}")).unwrap();
        let mut stream =
            comp.create_stream(format!("/storage{i}/data")).unwrap();
        stream.write_all(&[7u8; 8]).unwrap();
    }
    comp.flush().unwrap();

    fn check(comp: &mut CompoundFile<Cursor<Vec<u8>>>) {
        let walked: Vec<(String, bool)> = comp
            .walk()
            .filter(|e| !e.is_root())
            .map(|e| (e.path().to_string_lossy().into_owned(), e.is_stream()))
            .collect();
        assert!(walked.len() > 1500, "walked {} entries", walked.len());
        for (path, is_stream) in &walked {
            assert!(comp.exists(path), "{} exists", path);
            assert_eq!(comp.is_stream(path), *is_stream, "{path} kind");
            assert_eq!(comp.is_storage(path), !*is_stream, "{path} kind");
            let other_case = path.to_uppercase();
            assert!(comp.exists(&other_case), "{} exists", other_case);
            assert_eq!(comp.is_stream(&other_case), *is_stream);
        }
        for i in (0..600).step_by(4) {
            if i % 8 != 0 {
                assert!(!comp.exists(format!("/Storage{i}/Wide")), "removed");
                assert!(comp.is_stream(format!("/Storage{i}/Data")), "kept");
            }
        }
        for i in (0..600).step_by(8) {
            // "data" was created again under the new storage of the same
            // name; "Parameters" was not.
            assert!(
                !comp.exists(format!("/Storage{i}/Parameters")),
                "removed"
            );
            let mut data = Vec::new();
            comp.open_stream(format!("/STORAGE{i}/DATA"))
                .unwrap()
                .read_to_end(&mut data)
                .unwrap();
            assert_eq!(data, vec![7u8; 8], "re-added under another case");
        }
    }
    check(&mut comp);

    let bytes = comp.into_inner().into_inner();
    let mut reopened = CompoundFile::open(Cursor::new(bytes)).unwrap();
    check(&mut reopened);
}

/// A removed name can be created again, and a name that differs from an
/// existing one only in case is the same entry, not a second one.
#[test]
fn a_name_is_one_entry_in_any_case() {
    let mut comp = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    comp.create_storage("/Dir").unwrap();
    comp.create_stream("/Dir/File").unwrap().write_all(b"one").unwrap();
    assert!(comp.create_new_stream("/dir/FILE").is_err(), "same entry");
    comp.remove_stream("/DIR/file").unwrap();
    assert!(!comp.exists("/Dir/File"));
    comp.create_stream("/dir/file").unwrap().write_all(b"two").unwrap();
    let mut data = Vec::new();
    comp.open_stream("/Dir/FILE").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, b"two");
}
