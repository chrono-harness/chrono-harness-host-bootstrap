//! 本宿主的源码构建、共享启动和所有者升级；政策仍来自宿主登记。
#![allow(unexpected_cfgs)]

pub mod bindings;
pub mod construction;
pub mod download;
pub mod environment;
pub mod execution;
pub mod lifecycle;
pub mod original;
pub mod owner;
pub mod profile;
pub mod proved_build;
mod registered_methods;
#[cfg(not(verus_keep_ghost))]
pub mod report_query;
pub mod shared;
pub mod source;
pub mod verified_bindings;
mod verified_cargo;
