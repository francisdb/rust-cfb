use cfb::CompoundFile;
use std::io::{Cursor, Read, Write};

/// Writes `count` mini streams of `len` bytes each, named `s{i}`, whose
/// contents identify the stream.
fn write_streams(
    comp: &mut CompoundFile<Cursor<Vec<u8>>>,
    count: u32,
    len: usize,
) {
    for i in 0..count {
        let data = vec![(i % 251) as u8; len];
        let mut stream = comp.create_stream(format!("/s{i}")).unwrap();
        stream.write_all(&data).unwrap();
    }
}

fn assert_stream(
    comp: &mut CompoundFile<Cursor<Vec<u8>>>,
    i: u32,
    len: usize,
) {
    let mut data = Vec::new();
    comp.open_stream(format!("/s{i}"))
        .unwrap()
        .read_to_end(&mut data)
        .unwrap();
    assert_eq!(data.len(), len, "stream s{i} length");
    assert!(
        data.iter().all(|&b| b == (i % 251) as u8),
        "stream s{i} contents"
    );
}

/// Many small streams grow the mini stream across many regular sectors,
/// freeing some shrinks it, and adding more grows it again; every stream
/// must read back intact through all of it, whether the file is the one
/// being written or reopened from its bytes.
#[test]
fn many_mini_streams_grow_shrink_and_regrow() {
    let mut comp = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    write_streams(&mut comp, 800, 300);
    for i in 0..800 {
        assert_stream(&mut comp, i, 300);
    }

    // Free every third stream, then add a second batch.
    for i in (0..800).step_by(3) {
        comp.remove_stream(format!("/s{i}")).unwrap();
    }
    for i in 800..1000 {
        let data = vec![(i % 251) as u8; 300];
        let mut stream = comp.create_stream(format!("/s{i}")).unwrap();
        stream.write_all(&data).unwrap();
    }
    comp.flush().unwrap();

    let check = |comp: &mut CompoundFile<Cursor<Vec<u8>>>| {
        for i in 0..1000u32 {
            if i < 800 && i % 3 == 0 {
                assert!(!comp.is_stream(format!("/s{i}")), "s{i} was removed");
            } else {
                assert_stream(comp, i, 300);
            }
        }
    };
    check(&mut comp);

    // The same file reopened from its bytes.
    let bytes = comp.into_inner().into_inner();
    let mut reopened = CompoundFile::open(Cursor::new(bytes)).unwrap();
    check(&mut reopened);
}

/// A mini stream that grows past the mini-stream cutoff moves to the regular
/// sectors; a stream truncated back below it returns. Both transitions keep
/// the other mini streams readable.
#[test]
fn a_stream_crossing_the_mini_cutoff_keeps_its_neighbours_intact() {
    let mut comp = CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    write_streams(&mut comp, 100, 200);

    let mut big = comp.create_stream("/big").unwrap();
    big.write_all(&vec![7u8; 100]).unwrap();
    big.write_all(&vec![7u8; 10_000]).unwrap();
    big.set_len(50).unwrap();
    drop(big);

    for i in 0..100 {
        assert_stream(&mut comp, i, 200);
    }
    let mut data = Vec::new();
    comp.open_stream("/big").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data, vec![7u8; 50]);
}
