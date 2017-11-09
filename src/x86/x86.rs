//! x86-specific accelerated code.

#[cfg(feature = "x86_ssse3")]
#[path = "ssse3.rs"]
mod ssse3;

#[cfg(feature = "x86_ssse3")]
pub use self::ssse3::Ssse3;

#[cfg(feature = "x86_sse41")]
#[path = "sse41.rs"]
mod sse41;

#[cfg(feature = "x86_sse41")]
pub use self::sse41::Sse41;
