pub mod filter;
pub mod math;
pub mod protobuf;
pub mod result;
mod test;

pub use filter::CwJsonFilter;
pub use math::{gt_json, lt_json};
pub use protobuf::{base64_encode_protobuf, get_protobuf_messages};
pub use result::{FilterFailure, FilterFatalError, FilterResult};

pub use prost_reflect;

use base64::{
    alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};

/// Base64 decoding engine
pub const BASE64_ENGINE: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);
