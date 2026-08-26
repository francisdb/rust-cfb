use cfb::CompoundFile;
use std::io::{Cursor, Read, Write};

/// Removing a sibling with two subtrees fills its slot with its in-order
/// predecessor. When that predecessor is a storage, the storage's children
/// move with it and must stay reachable by path — before and after the file
/// is reopened.
#[test]
fn a_storage_moved_into_a_removed_siblings_slot_keeps_its_children() {
    let mut comp = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    // "foo" is the first child; "baz" sorts before it (left), "quux" after
    // it (right), so removing "foo" moves "baz" — and its children.
    comp.create_storage("/foo").unwrap();
    comp.create_storage("/baz").unwrap();
    comp.create_storage("/quux").unwrap();
    comp.create_storage("/baz/inner").unwrap();
    comp.create_stream("/baz/blarg").unwrap().write_all(b"blarg").unwrap();
    comp.create_stream("/baz/inner/deep").unwrap().write_all(b"deep").unwrap();
    comp.remove_storage("/foo").unwrap();

    fn check(comp: &mut CompoundFile<Cursor<Vec<u8>>>) {
        assert!(!comp.exists("/foo"));
        assert!(comp.is_storage("/baz"));
        assert!(comp.is_storage("/quux"));
        assert!(comp.is_storage("/baz/inner"));
        let mut data = Vec::new();
        comp.open_stream("/BAZ/blarg")
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"blarg");
        data.clear();
        comp.open_stream("/baz/inner/DEEP")
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"deep");
    }
    check(&mut comp);
    // The moved storage can be edited under its new slot.
    comp.remove_stream("/baz/blarg").unwrap();
    assert!(!comp.exists("/baz/blarg"));
    comp.create_stream("/baz/blarg").unwrap().write_all(b"blarg").unwrap();
    check(&mut comp);

    let bytes = comp.into_inner().into_inner();
    let mut reopened = CompoundFile::open_strict(Cursor::new(bytes)).unwrap();
    check(&mut reopened);
}
