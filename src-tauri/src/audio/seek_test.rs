use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, Source};

fn main() {
    let file = File::open("/home/ritwikg/Music/test.mp3").unwrap_or_else(|_| File::open("/dev/null").unwrap());
    let reader = BufReader::new(file);
    let mut decoder = Decoder::new(reader).unwrap();
    println!("Can seek? {:?}", decoder.try_seek(std::time::Duration::from_secs(10)));
}
