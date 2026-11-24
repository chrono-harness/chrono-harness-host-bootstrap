//! 显式环境选择与 Rustup 调用；不修改进程全局环境或全局默认工具链。
#[cfg(verus_keep_ghost)]
use super::{environment_key, environment_value, host_variable, valid_environment_key};
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings as policy;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::{environment_key, environment_value, host_variable};
use vstd::prelude::*;

verus! {

pub fn explicit_binding(key: &[u8], value: &[u8]) -> (valid: bool)
    ensures valid == (valid_environment_key(key@)
        && forall|i: int| 0 <= i < value.len() ==> value[i] != 0),
{
    environment_key(key) && environment_value(value)
}

// The erased index identifies the first failure in the proof.
pub enum HostSelectionError<E> { Empty, Key(Ghost<int>), Path(Ghost<int>, E) }

pub open spec fn allowed(row: (String, String, bool)) -> bool {
    !row.2 && (row.0@ == "HOME"@ || row.0@ == "CARGO_HOME"@ || row.0@ == "TMPDIR"@)
}

pub open spec fn resolved<P, E, F>(entry: (String, String, bool), path: P, resolve: F) -> bool
    where F: Fn(&String) -> Result<P, E>,
{
    allowed(entry) && resolve.ensures((&entry.1,), Ok::<P, E>(path))
}

pub open spec fn could_resolve<P, E, F>(entry: (String, String, bool), resolve: F) -> bool
    where F: Fn(&String) -> Result<P, E>,
{
    exists|path: P| #[trigger] resolved(entry, path, resolve)
}

pub open spec fn prior<P, E, F>(rows: &[(String, String, bool)], i: int, resolve: F) -> bool
    where F: Fn(&String) -> Result<P, E>,
{
    0 <= i < rows.len()
        && forall|j: int| 0 <= j < i ==> #[trigger] could_resolve(rows[j], resolve)
}

pub open spec fn host_outcome<P, E, F>(rows: &[(String, String, bool)], resolve: F, out: Result<Vec<P>, HostSelectionError<E>>) -> bool
    where F: Fn(&String) -> Result<P, E>,
{
    match out {
        Ok(paths) => rows.len() > 0 && paths.len() == rows.len()
                && forall|i: int| 0 <= i < rows.len() ==> resolved(rows[i], paths[i], resolve),
        Err(HostSelectionError::Empty) => rows.len() == 0,
        Err(HostSelectionError::Key(i)) => prior(rows, i@, resolve) && !allowed(rows[i@]),
        Err(HostSelectionError::Path(i, e)) => prior(rows, i@, resolve) && allowed(rows[i@])
                && resolve.ensures((&rows[i@].1,), Err::<P, E>(e)),
    }
}

/// Return paths in the same order as these entries once every admission and path check succeeds.
fn selected_paths<P, E, F>(rows: &[(String, String, bool)], resolve: F)
    -> (out: Result<Vec<P>, HostSelectionError<E>>)
    where F: Fn(&String) -> Result<P, E>,
    requires forall|value: &String| resolve.requires((value,)),
    ensures host_outcome(rows, resolve, out),
{
    let mut paths = Vec::new();
    if rows.len() == 0 { return Err(HostSelectionError::Empty); }
    let mut i = 0usize;
    while i < rows.len()
        invariant
            i <= rows.len(), rows.len() > 0, paths.len() == i,
            forall|value: &String| resolve.requires((value,)),
            forall|j: int| 0 <= j < i ==> resolved(rows[j], paths[j], resolve),
            forall|j: int| 0 <= j < i ==> #[trigger] could_resolve(rows[j], resolve),
        decreases rows.len() - i,
    {
        let row = &rows[i];
        assert(prior(rows, i as int, resolve));
        if !host_variable(&row.0) || row.2 {
            return Err(HostSelectionError::Key(Ghost(i as int)));
        }
        assert(allowed(rows[i as int]));
        match resolve(&row.1) {
            Err(e) => {
                return Err(HostSelectionError::Path(Ghost(i as int), e));
            },
            Ok(path) => {
                assert(resolved(*row, path, resolve));
                assert(could_resolve(*row, resolve));
                paths.push(path);
            },
        }
        i += 1;
    }
    Ok(paths)
}

pub fn host_paths<P, E, F>(entries: Option<&[(String, String, bool)]>, resolve: F)
    -> (out: Result<Vec<P>, HostSelectionError<E>>)
    where F: Fn(&String) -> Result<P, E>,
    requires forall|value: &String| resolve.requires((value,)),
    ensures match entries {
        None => match out { Ok(paths) => paths.len() == 0, _ => false },
        Some(rows) => host_outcome(rows, resolve, out),
    },
{
    match entries { None => Ok(Vec::new()), Some(rows) => selected_paths(rows, resolve) }
}

}

#[cfg(not(verus_keep_ghost))]
use crate::codec::json;
#[cfg(not(verus_keep_ghost))]
use crate::files::resolve_program;
#[cfg(not(verus_keep_ghost))]
use crate::profile::path as host_path;
#[cfg(not(verus_keep_ghost))]
use crate::{
    bindings::{self, Declarations, Environment, NativePlatform, Tools},
    verified_bindings::Executable,
};
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitStatus,
};

#[derive(Debug)]
#[cfg(not(verus_keep_ghost))]
pub enum Failure {
    Selection(String),
    Installer(ExitStatus),
}

#[cfg(not(verus_keep_ghost))]
impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Selection(message)
    }
}
#[cfg(not(verus_keep_ghost))]
impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Self::Selection(message.into())
    }
}
#[cfg(not(verus_keep_ghost))]
impl std::fmt::Display for Failure {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Selection(message) => out.write_str(message),
            Self::Installer(status) => write!(out, "Rustup failed: {status}"),
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub struct Prepared {
    pub environment: Environment,
    pub host_directories: Vec<PathBuf>,
    toolchain: String,
    incremental: Option<bool>,
}

#[derive(Debug)]
#[cfg(not(verus_keep_ghost))]
pub struct Selected {
    pub environment: Environment,
    pub versions: Tools<String>,
    pub bindings: Option<Tools<Executable>>,
}

#[cfg(not(verus_keep_ghost))]
fn typed<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
impl Prepared {
    /// 只解析与观察声明的构建环境；不创建目录或启动进程。
    pub fn parse(root: &Path, bytes: &[u8], inherited: &Environment) -> Result<Self, String> {
        let config = json(bytes)?;
        let toolchain: String = typed(&config["rust_toolchain"])?;
        let incremental = config
            .get("rust_incremental")
            .map(typed::<bool>)
            .transpose()?;
        let mut environment = inherited.clone();
        let explicit = config
            .get("environment")
            .map(typed::<BTreeMap<String, String>>)
            .transpose()?
            .unwrap_or_default();
        for (key, value) in &explicit {
            if !explicit_binding(key.as_bytes(), value.as_bytes()) {
                return Err("invalid bootstrap environment selection".into());
            }
            environment.insert(key.into(), value.into());
        }
        let host = config
            .get("host_environment")
            .map(typed::<BTreeMap<String, String>>)
            .transpose()?;
        let entries = host.map(|values| {
            values
                .into_iter()
                .map(|(key, value)| {
                    let conflict = explicit.contains_key(&key);
                    (key, value, conflict)
                })
                .collect::<Vec<_>>()
        });
        let host_directories = host_paths(entries.as_deref(), |value| {
            host_path(root, value, b".chrono-harness/cache/")
        })
        .map_err(|error| match error {
            HostSelectionError::Empty => "empty bootstrap host environment selection".to_string(),
            HostSelectionError::Key(_) => {
                "invalid or conflicting bootstrap host environment selection".to_string()
            }
            HostSelectionError::Path(_, error) => error,
        })?;
        if let Some(entries) = entries {
            for ((key, _, _), path) in entries.into_iter().zip(&host_directories) {
                environment.insert(key.into(), path.as_os_str().to_owned());
            }
        }
        Ok(Self {
            environment,
            host_directories,
            toolchain,
            incremental,
        })
    }

    /// 调用方先完成显式 prerequisites，再选择工具链及补齐缺少的组件。
    /// 不创建 host_directories；目录建立仍由源码构建调用方负责。
    pub fn activate(
        self,
        root: &Path,
        declarations: Option<&Declarations>,
        configured: &BTreeMap<String, String>,
    ) -> Result<Selected, Failure> {
        let mut environment = self.environment;
        let path = environment
            .get(OsStr::new("PATH"))
            .map(|v| v.to_str().ok_or("non UTF-8 PATH"))
            .transpose()?;
        let rustup = resolve_program(root, "rustup", path)?;
        let install = |args: &[&str]| -> Result<(), Failure> {
            let status = bindings::command(root, &rustup, args, &environment)?
                .status()
                .map_err(|error| error.to_string())?;
            if !status.success() {
                return Err(Failure::Installer(status));
            }
            Ok(())
        };
        let probe = |name: &str| {
            bindings::capture(
                root,
                &rustup,
                &["run", &self.toolchain, name, "--version"],
                &environment,
            )
        };
        for (name, cargo) in [("cargo", true), ("rustfmt", false)] {
            match policy::probe_action(probe(name)?.status.success(), cargo) {
                policy::ProbeAction::Ready => {}
                policy::ProbeAction::InstallMinimal => install(&[
                    "toolchain",
                    "install",
                    &self.toolchain,
                    "--profile",
                    "minimal",
                    "--component",
                    "rustfmt",
                ])?,
                policy::ProbeAction::AddFormat => install(&[
                    "component",
                    "add",
                    "--toolchain",
                    &self.toolchain,
                    "rustfmt",
                ])?,
            }
        }
        environment.insert("RUSTUP_TOOLCHAIN".into(), self.toolchain.clone().into());
        environment.insert("CARGO_TERM_COLOR".into(), "never".into());
        if let Some(incremental) = self.incremental {
            environment.insert(
                "CARGO_INCREMENTAL".into(),
                if incremental { "1" } else { "0" }.into(),
            );
        }
        let bindings = declarations
            .map(|declared| {
                declared.select(root, &rustup, &self.toolchain, configured, &environment)
            })
            .transpose()?;
        let version = |name: &str, selected: Option<&Executable>| -> Result<String, String> {
            let path = environment
                .get(OsStr::new("PATH"))
                .map(|v| v.to_str().ok_or("non UTF-8 PATH"))
                .transpose()?;
            let executable = match selected {
                Some(value) => PathBuf::from(&value.path),
                None => resolve_program(root, name, path)?,
            };
            bindings::output_text(
                bindings::capture(root, &executable, &["--version"], &environment)?,
                name,
            )
        };
        let versions = Tools {
            cargo: version("cargo", bindings.as_ref().map(|value| &value.cargo))?,
            rustc: version("rustc", bindings.as_ref().map(|value| &value.rustc))?,
        };
        let expected = format!("rustc {}", self.toolchain);
        if !policy::version_matches(versions.rustc.as_bytes(), expected.as_bytes(), false) {
            return Err("unexpected rustc version".into());
        }
        if let Some(selected) = &bindings {
            environment.insert("RUSTC".into(), selected.rustc.path.clone().into());
            environment.insert("RUSTC_WRAPPER".into(), "".into());
            environment.insert("RUSTC_WORKSPACE_WRAPPER".into(), "".into());
        }
        Ok(Selected {
            environment,
            versions,
            bindings,
        })
    }
}

/// 先解释绑定和全部环境，再允许安装；固定注册表字节由调用方提供。
#[cfg(not(verus_keep_ghost))]
pub fn select(
    root: &Path,
    config: &[u8],
    native: &NativePlatform,
    registry: Option<&[u8]>,
    inherited: &Environment,
    configured: &BTreeMap<String, String>,
) -> Result<Selected, Failure> {
    let declarations = Declarations::parse(config, native, registry)?;
    Prepared::parse(root, config, inherited)?.activate(root, declarations.as_ref(), configured)
}
