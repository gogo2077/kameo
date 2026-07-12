use std::io;

use hmac::{Hmac, Mac};
use sha2::Sha256;

pub(super) const AUTH_MAGIC: &[u8; 4] = b"KCON";
pub(super) const AUTH_VERSION: u8 = 1;
pub(super) const AUTH_NONCE_LEN: usize = 32;
pub(super) const AUTH_TAG_LEN: usize = 32;
pub(super) const AUTH_CHALLENGE_LEN: usize = AUTH_MAGIC.len() + 1 + AUTH_NONCE_LEN;
pub(super) const AUTH_OK: u8 = 1;
pub(super) const AUTH_DENIED: u8 = 0;
pub(super) const MAX_FRAME_BYTES: u32 = 64 * 1024 * 1024;

type HmacSha256 = Hmac<Sha256>;

pub(super) fn challenge(nonce: &[u8; AUTH_NONCE_LEN]) -> [u8; AUTH_CHALLENGE_LEN] {
    let mut challenge = [0; AUTH_CHALLENGE_LEN];
    challenge[..AUTH_MAGIC.len()].copy_from_slice(AUTH_MAGIC);
    challenge[AUTH_MAGIC.len()] = AUTH_VERSION;
    challenge[AUTH_MAGIC.len() + 1..].copy_from_slice(nonce);
    challenge
}

pub(super) fn parse_challenge(
    challenge: &[u8; AUTH_CHALLENGE_LEN],
) -> io::Result<[u8; AUTH_NONCE_LEN]> {
    if &challenge[..AUTH_MAGIC.len()] != AUTH_MAGIC || challenge[AUTH_MAGIC.len()] != AUTH_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid console authentication challenge",
        ));
    }

    let mut nonce = [0; AUTH_NONCE_LEN];
    nonce.copy_from_slice(&challenge[AUTH_MAGIC.len() + 1..]);
    Ok(nonce)
}

pub(super) fn tag(token: &[u8], nonce: &[u8; AUTH_NONCE_LEN]) -> [u8; AUTH_TAG_LEN] {
    let mut mac = HmacSha256::new_from_slice(token).expect("HMAC accepts keys of any size");
    mac.update(nonce);
    mac.finalize().into_bytes().into()
}

pub(super) fn verify(token: &[u8], nonce: &[u8; AUTH_NONCE_LEN], tag: &[u8; AUTH_TAG_LEN]) -> bool {
    let mut mac = HmacSha256::new_from_slice(token).expect("HMAC accepts keys of any size");
    mac.update(nonce);
    mac.verify_slice(tag).is_ok()
}
