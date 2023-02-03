pub mod sync {
    include!(concat!(env!("OUT_DIR"), "/bolik_sync.rs"));
}

pub use prost;
pub use prost_types;
