use chrono_host_bootstrap::native_scratch::{maintain, run};
use std::{
    ffi::{OsStr, OsString},
    fs::{self, File},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tempfile::TempDir;

struct Host {
    root: TempDir,
    git: PathBuf,
    child: PathBuf,
}
impl Host {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let git = root.path().join("git");
        let child = root.path().join("fixture-native");
        for name in [&git, &child] {
            symlink(env!("CARGO_BIN_EXE_chrono-test-bootstrap-tool"), name).unwrap();
        }
        Self { root, git, child }
    }
    fn run(&self, code: &str) -> u8 {
        run(
            self.root.path(),
            self.child.as_os_str(),
            &[OsString::from(code)],
            self.git.as_os_str(),
        )
        .unwrap()
    }
    fn business(&self) -> PathBuf {
        self.root.path().join("target/native-checks")
    }
    fn remaining(&self) -> Vec<PathBuf> {
        fs::read_dir(self.business())
            .unwrap()
            .map(|r| r.unwrap().path())
            .collect()
    }
    fn maintain(&self) -> usize {
        maintain(self.root.path(), 2, self.git.as_os_str()).unwrap()
    }
}
fn expire(p: &Path) {
    File::open(p)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(10))
        .unwrap();
}
#[test]
fn success_releases_and_failure_preserves_until_expired_without_readers() {
    let h = Host::new();
    assert_eq!(h.maintain(), 0);
    assert_eq!(h.run("0"), 0);
    assert!(h.remaining().is_empty());
    assert_eq!(h.run("42"), 42);
    let p = h.remaining().pop().unwrap();
    assert_eq!(fs::read(p.join("object")).unwrap(), b"compiled");
    assert_eq!(h.maintain(), 0);
    let reader = File::open(&p).unwrap();
    reader.lock_shared().unwrap();
    expire(&p);
    assert_eq!(h.maintain(), 0);
    drop(reader);
    assert_eq!(h.maintain(), 1);
    assert!(!p.exists());
}
#[test]
fn tracked_replaced_and_linked_paths_are_preserved() {
    let h = Host::new();
    assert_eq!(h.run("9"), 9);
    let p = h.remaining().pop().unwrap();
    expire(&p);
    let name = p.strip_prefix(h.root.path()).unwrap().join("source");
    let mut tracked = name.as_os_str().as_encoded_bytes().to_vec();
    tracked.push(0);
    fs::write(h.root.path().join("tracked"), tracked).unwrap();
    assert_eq!(h.maintain(), 0);
    fs::remove_file(h.root.path().join("tracked")).unwrap();
    let outside = h.root.path().join("original");
    fs::rename(&p, &outside).unwrap();
    symlink(&outside, &p).unwrap();
    assert_eq!(h.maintain(), 0);
    assert!(outside.join("object").exists());
    fs::remove_file(&p).unwrap();
    fs::create_dir(&p).unwrap();
    fs::copy(
        outside.join(".chrono-native-owner"),
        p.join(".chrono-native-owner"),
    )
    .unwrap();
    fs::write(p.join("unexpected"), "keep").unwrap();
    expire(&p);
    assert_eq!(h.maintain(), 0);
    assert!(p.join("unexpected").exists());
}
#[test]
fn missing_git_and_symlinked_business_fail_before_removal() {
    let h = Host::new();
    assert_eq!(h.run("1"), 1);
    let p = h.remaining().pop().unwrap();
    expire(&p);
    assert!(maintain(h.root.path(), 1, OsStr::new("/missing-git-for-test")).is_err());
    assert!(p.exists());
    let old = h.root.path().join("elsewhere");
    fs::rename(h.business(), &old).unwrap();
    symlink(old, h.business()).unwrap();
    assert!(maintain(h.root.path(), 1, h.git.as_os_str()).is_err());
    assert!(maintain(h.root.path(), 0, h.git.as_os_str()).is_err());
}
