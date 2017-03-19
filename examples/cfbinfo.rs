extern crate cfb;

use std::env;
use std::io::Read;

fn main() {
    if env::args().count() != 2 {
        println!("Usage: cfbinfo <path>");
        return;
    }
    let path = env::args().nth(1).unwrap();
    let mut comp = cfb::open(path).unwrap();
    let mut stack = vec![comp.entry("/").unwrap()];
    while let Some(entry) = stack.pop() {
        println!("{:?} ({} bytes)", entry.path(), entry.len());
        if entry.is_storage() {
            stack.extend(comp.read_storage(entry.path()).unwrap());
        }
        if entry.is_stream() {
            let mut stream = comp.open_stream(entry.path()).unwrap();
            debug_assert_eq!(stream.len(), entry.len());
            let mut data = Vec::new();
            stream.read_to_end(&mut data).unwrap();
            debug_assert_eq!(data.len() as u64, entry.len());
        }
    }
}
