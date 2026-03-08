pub mod crypto;
pub mod error;

pub use error::CloakError;

pub type Result<T> = std::result::Result<T, CloakError>;
