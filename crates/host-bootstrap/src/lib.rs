//! 本宿主的源码构建、共享启动和所有者升级；政策仍来自宿主登记。
#![allow(unexpected_cfgs)]

pub mod bindings;
#[cfg(not(verus_keep_ghost))]
pub mod cargo_proofs;
#[cfg(not(verus_keep_ghost))]
mod codec;
#[cfg(not(verus_keep_ghost))]
pub mod compiler;
pub mod construction;
#[cfg(not(verus_keep_ghost))]
mod descriptors;
pub mod download;
pub mod environment;
pub mod execution;
#[cfg(not(verus_keep_ghost))]
pub mod files;
pub mod lifecycle;
pub mod original;
pub mod owner;
#[cfg(not(verus_keep_ghost))]
pub mod owner_proof;
pub mod profile;
pub mod proved_build;
mod registered_methods;
#[cfg(not(verus_keep_ghost))]
pub mod report_query;
#[cfg(not(verus_keep_ghost))]
mod retention;
pub mod shared;
pub mod source;
pub mod template;
pub mod verified_bindings;
pub mod verified_cargo;
