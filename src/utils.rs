use sha1::{Digest, Sha1};

pub(crate) fn compute_hash(bytes: &Vec<u8>) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    hasher.finalize().into()
}
