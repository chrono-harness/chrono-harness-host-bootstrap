//! A runner session holds the actual host leases while bootstrap builds or reads originals.
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    io::{BufRead, BufReader, Write},
    net::Shutdown,
    os::fd::{AsFd, OwnedFd},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
    rc::{Rc, Weak},
};
type Client = Rc<RefCell<Session>>;
thread_local! {static SESSIONS: RefCell<BTreeMap<PathBuf,Weak<RefCell<Session>>>> = RefCell::new(BTreeMap::new());}
struct Session {
    socket: UnixStream,
    output: BufReader<ChildStdout>,
    child: Child,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Write);
        let _ = self.child.wait();
    }
}
impl Session {
    fn open(root: &Path) -> Result<Client, String> {
        SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            if let Some(client) = sessions.get(root).and_then(Weak::upgrade) {
                return Ok(client);
            }
            let (socket, input) = UnixStream::pair().map_err(|e| e.to_string())?;
            let mut child = Command::new("chrono-harness")
                .arg("retained-artifacts-v1")
                .current_dir(root)
                .stdin(Stdio::from(OwnedFd::from(input)))
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| format!("retained-artifacts-v1: {e}"))?;
            let output = BufReader::new(child.stdout.take().expect("piped session response"));
            let session = Rc::new(RefCell::new(Session {
                socket,
                output,
                child,
            }));
            sessions.retain(|_, weak| weak.strong_count() > 0);
            sessions.insert(root.to_owned(), Rc::downgrade(&session));
            Ok(session)
        })
    }
    fn call<T: serde::de::DeserializeOwned>(&mut self, op: Value) -> Result<T, String> {
        serde_json::to_writer(&mut self.socket, &op).map_err(|e| e.to_string())?;
        self.socket.write_all(b"\n").map_err(|e| e.to_string())?;
        let mut line = Vec::new();
        if self
            .output
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            let status = self.child.wait().map_err(|e| e.to_string())?;
            return Err(format!(
                "retained-artifacts-v1 ended without a response: {status}"
            ));
        }
        serde_json::from_slice::<Result<T, String>>(&line).map_err(|e| e.to_string())?
    }
    fn publication<T: serde::de::DeserializeOwned>(&mut self, action: Value) -> Result<T, String> {
        self.call(json!({"operation":"publication","action":action}))
    }
    fn append_descriptors(&mut self, result: &mut Vec<OwnedFd>) -> Result<(), String> {
        use rustix::net::{
            RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags, recvmsg,
        };
        use std::{io::IoSliceMut, mem::MaybeUninit};
        const BATCH: usize = 32;
        let mut offset = 0;
        loop {
            let n: usize = self.call(json!({"operation":"scoped-descriptors","offset":offset}))?;
            if n == 0 {
                return Ok(());
            }
            if n > BATCH {
                return Err("invalid retained descriptor count".into());
            }
            let mut marker = [0u8];
            let mut buffer = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(BATCH))];
            let mut controls = RecvAncillaryBuffer::new(&mut buffer);
            #[cfg(target_os = "linux")]
            let flags = RecvFlags::CMSG_CLOEXEC;
            #[cfg(not(target_os = "linux"))]
            let flags = RecvFlags::empty();
            let msg = recvmsg(
                &self.socket,
                &mut [IoSliceMut::new(&mut marker)],
                &mut controls,
                flags,
            )
            .map_err(|e| e.to_string())?;
            if msg.bytes != 1 || marker != [1] || msg.flags.contains(ReturnFlags::CTRUNC) {
                return Err("invalid retained descriptor transfer".into());
            }
            let before = result.len();
            for control in controls.drain() {
                if let RecvAncillaryMessage::ScmRights(fds) = control {
                    for fd in fds {
                        rustix::io::fcntl_setfd(fd.as_fd(), rustix::io::FdFlags::CLOEXEC)
                            .map_err(|e| e.to_string())?;
                        result.push(fd);
                    }
                }
            }
            if result.len() - before != n {
                return Err("incomplete retained descriptor transfer".into());
            }
            if n < BATCH {
                return Ok(());
            }
            offset += n;
        }
    }
}
pub(crate) fn active_descriptors() -> Result<Vec<OwnedFd>, String> {
    SESSIONS.with(|sessions| {
        let mut fds = Vec::new();
        for client in sessions.borrow().values().filter_map(Weak::upgrade) {
            client.borrow_mut().append_descriptors(&mut fds)?;
        }
        Ok(fds)
    })
}
pub struct Scope {
    client: Client,
}
pub fn scope(root: &Path) -> Result<Scope, String> {
    Ok(Scope {
        client: Session::open(root)?,
    })
}
fn relative<'a>(root: &Path, path: &'a Path) -> Result<&'a str, String> {
    path.strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .ok_or_else(|| format!("invalid host-relative path: {}", path.display()))
}
pub struct Publication {
    scope: Scope,
    path: String,
    open: bool,
}
impl Drop for Publication {
    fn drop(&mut self) {
        if self.open {
            let _: Result<(), _> = self
                .scope
                .client
                .borrow_mut()
                .publication(json!({"kind":"close","path":self.path}));
        }
    }
}
impl Scope {
    pub fn create(
        &self,
        root: &Path,
        parent: &Path,
        prefix: &str,
        producer: &str,
        outcome: Value,
    ) -> Result<(PathBuf, Option<Publication>), String> {
        #[derive(serde::Deserialize)]
        struct Created {
            path: String,
            adopted: bool,
        }
        let created:Created=self.client.borrow_mut().publication(json!({"kind":"create","parent":relative(root,parent)?,"prefix":prefix,"producer":producer,"outcome":outcome}))?;
        let directory = root.join(&created.path);
        Ok((
            directory,
            created.adopted.then(|| Publication {
                scope: Scope {
                    client: self.client.clone(),
                },
                path: created.path,
                open: true,
            }),
        ))
    }
    pub fn diagnostic(&self, path: &str) -> Result<(), String> {
        self.client
            .borrow_mut()
            .publication(json!({"kind":"retain-diagnostic","path":path}))
    }
    pub fn dependencies(&self, path: &str, originals: &[String]) -> Result<(), String> {
        self.client
            .borrow_mut()
            .call(json!({"operation":"dependencies","path":path,"originals":originals}))
    }
    pub fn reference(&self, slot: &str, path: &str, inputs: &[String]) -> Result<(), String> {
        self.client
            .borrow_mut()
            .publication(json!({"kind":"reference","slot":slot,"path":path,"inputs":inputs}))
    }
    pub fn settle(&self, slot: &str, path: &str) -> Result<(), String> {
        self.client
            .borrow_mut()
            .publication(json!({"kind":"settle","slot":slot,"path":path}))
    }
    pub fn read(&self, path: &str, slot: Option<&str>) -> Result<Option<Read>, String> {
        if let Some(slot) = slot {
            let token: Option<u64> = self
                .client
                .borrow_mut()
                .publication(json!({"kind":"read-selected","path":path,"slot":slot}))?;
            return Ok(token.map(|token| Read {
                client: self.client.clone(),
                token,
            }));
        }
        self.client
            .borrow_mut()
            .call::<()>(json!({"operation":"retain-read","path":path}))?;
        Ok(None)
    }
}
pub struct Read {
    client: Client,
    token: u64,
}
impl Drop for Read {
    fn drop(&mut self) {
        let _: Result<(), _> = self
            .client
            .borrow_mut()
            .publication(json!({"kind":"release-read","token":self.token}));
    }
}
pub fn enroll(
    root: &Path,
    producer: &str,
    directory: &Path,
    outcome: Value,
) -> Result<Option<Publication>, String> {
    let scope = scope(root)?;
    let path = relative(root, directory)?.to_owned();
    let adopted: bool = scope
        .client
        .borrow_mut()
        .publication(json!({"kind":"enroll","path":path,"producer":producer,"outcome":outcome}))?;
    Ok(adopted.then_some(Publication {
        scope,
        path,
        open: adopted,
    }))
}
impl Publication {
    fn name<'a>(&self, path: &'a Path) -> Result<&'a str, String> {
        let parent = path.parent().ok_or("missing publication parent")?;
        if !parent.ends_with(&self.path) {
            return Err("output does not belong to publication".into());
        }
        path.file_name()
            .and_then(|s| s.to_str())
            .ok_or("invalid publication filename".into())
    }
    pub fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        let destination:PathBuf=self.scope.client.borrow_mut().publication(json!({"kind":"reserve-write","path":self.path,"name":self.name(path)?,"length":bytes.len()}))?;
        if destination != path {
            return Err("retained write destination changed".into());
        }
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(destination)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())
    }
    pub fn copy_file(&self, source: &Path, target: &Path) -> Result<(), String> {
        self.scope.client.borrow_mut().publication(
            json!({"kind":"copy","path":self.path,"source":source,"name":self.name(target)?}),
        )
    }
    pub fn accept_consumer(&self, consumer: &Value) -> Result<(), String> {
        self.scope
            .client
            .borrow_mut()
            .publication(json!({"kind":"accept-consumer","path":self.path,"consumer":consumer}))
    }
    pub fn protect_consumer(&self, consumer: &str) -> Result<Value, String> {
        self.scope
            .client
            .borrow_mut()
            .publication(json!({"kind":"protect-consumer","path":self.path,"consumer":consumer}))
    }
    pub fn complete(mut self, outcome: Value) -> Result<(), String> {
        self.open = false;
        self.scope
            .client
            .borrow_mut()
            .publication(json!({"kind":"complete","path":self.path,"outcome":outcome}))
    }
}
