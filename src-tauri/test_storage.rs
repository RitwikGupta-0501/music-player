use stream_download::storage::bounded::BoundedStorageProvider;
use stream_download::storage::memory::MemoryStorageProvider;
use std::num::NonZeroUsize;

fn main() {
    let _storage = BoundedStorageProvider::new(MemoryStorageProvider, NonZeroUsize::new(5 * 1024 * 1024).unwrap());
}
