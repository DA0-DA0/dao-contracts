use base64::{
    alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};

// Helper to reduce path allocations
#[inline]
pub fn append_path(base: &str, segment: &str) -> String {
    let mut path = String::with_capacity(base.len() + segment.len() + 1);
    path.push_str(base);
    path.push('.');
    path.push_str(segment);
    path
}

#[inline]
pub fn append_array_path(base: &str, index: usize) -> String {
    let mut path = String::with_capacity(base.len() + 10); // reasonable for most indices
    path.push_str(base);
    path.push('[');
    path.push_str(&index.to_string());
    path.push(']');
    path
}

/// Base64 decoding engine
pub const BASE64_ENGINE: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);
