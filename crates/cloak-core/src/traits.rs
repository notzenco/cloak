use crate::Result;

/// Embeds a payload into cover image data, returning the stego image bytes.
pub trait Encoder {
    fn encode(&self, cover: &[u8], payload: &[u8]) -> Result<Vec<u8>>;
}

/// Extracts a hidden payload from stego image data.
pub trait Decoder {
    fn decode(&self, stego: &[u8]) -> Result<Vec<u8>>;
}

/// Reports the maximum payload capacity of an image in bytes.
pub trait Capacity {
    fn capacity(&self, cover: &[u8]) -> Result<usize>;
}
