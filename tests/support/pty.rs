use std::{
    fs::File,
    io::{self, Read, Write},
    mem::MaybeUninit,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::process::CommandExt,
    },
    process::{Child, Command, ExitStatus, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TermiosSnapshot {
    input_flags: libc::tcflag_t,
    output_flags: libc::tcflag_t,
    control_flags: libc::tcflag_t,
    local_flags: libc::tcflag_t,
    control_chars: [libc::cc_t; libc::NCCS],
    input_speed: libc::speed_t,
    output_speed: libc::speed_t,
}

#[derive(Debug)]
pub(crate) struct PtyOutput {
    pub(crate) status: ExitStatus,
    pub(crate) bytes: Vec<u8>,
    pub(crate) before: TermiosSnapshot,
    pub(crate) after: TermiosSnapshot,
}

pub(crate) struct PtyChild {
    child: Child,
    master: File,
    slave: Option<OwnedFd>,
    reader: Option<JoinHandle<io::Result<Vec<u8>>>>,
    before: TermiosSnapshot,
    finished: bool,
}

impl PtyChild {
    pub(crate) fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.master.write_all(bytes)?;
        self.master.flush()
    }

    pub(crate) fn wait(&mut self, timeout: Duration) -> io::Result<PtyOutput> {
        let status = wait_with_timeout(&mut self.child, timeout)?;
        let after = termios_snapshot(
            self.slave
                .as_ref()
                .expect("PTY slave must remain open through terminal-state inspection")
                .as_raw_fd(),
        )?;
        self.slave.take();
        let bytes = self
            .reader
            .take()
            .expect("PTY reader must exist until child completion")
            .join()
            .map_err(|_| io::Error::other("PTY reader thread panicked"))??;
        self.finished = true;
        Ok(PtyOutput {
            status,
            bytes,
            before: self.before.clone(),
            after,
        })
    }
}

impl Drop for PtyChild {
    fn drop(&mut self) {
        if !self.finished {
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = self.child.kill();
            }
            let _ = self.child.wait();
            self.slave.take();
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
        }
    }
}

pub(crate) fn spawn(mut command: Command) -> io::Result<PtyChild> {
    let (master_fd, slave_fd) = open_pty()?;
    let before = termios_snapshot(slave_fd.as_raw_fd())?;

    command
        .stdin(Stdio::from(File::from(slave_fd.try_clone()?)))
        .stdout(Stdio::from(File::from(slave_fd.try_clone()?)))
        .stderr(Stdio::from(File::from(slave_fd.try_clone()?)));

    // SAFETY: this closure executes in the forked child immediately before exec. It only calls
    // async-signal-safe session/ioctl operations and does not touch shared Rust state.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            if libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY as libc::c_ulong, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let child = command.spawn()?;
    let master = File::from(master_fd);
    let mut reader_file = master.try_clone()?;
    let reader = thread::spawn(move || read_master(&mut reader_file));

    Ok(PtyChild {
        child,
        master,
        slave: Some(slave_fd),
        reader: Some(reader),
        before,
        finished: false,
    })
}

fn open_pty() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut master = -1;
    let mut slave = -1;
    let size = libc::winsize {
        ws_row: 24,
        ws_col: 80,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    // SAFETY: openpty initializes the two raw descriptors on success. The pointers are valid for
    // the duration of the call, and the resulting descriptors are immediately wrapped in OwnedFd.
    let result = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            &size,
        )
    };
    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: successful openpty returned two fresh owned file descriptors.
    Ok(unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) })
}

fn termios_snapshot(fd: libc::c_int) -> io::Result<TermiosSnapshot> {
    let mut value = MaybeUninit::<libc::termios>::uninit();
    // SAFETY: tcgetattr writes one initialized termios value to the valid out pointer.
    if unsafe { libc::tcgetattr(fd, value.as_mut_ptr()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: tcgetattr succeeded, so the value is initialized.
    let value = unsafe { value.assume_init() };
    // SAFETY: cfget*speed only reads the initialized termios structure.
    let input_speed = unsafe { libc::cfgetispeed(&value) };
    let output_speed = unsafe { libc::cfgetospeed(&value) };
    Ok(TermiosSnapshot {
        input_flags: value.c_iflag,
        output_flags: value.c_oflag,
        control_flags: value.c_cflag,
        local_flags: value.c_lflag,
        control_chars: value.c_cc,
        input_speed,
        output_speed,
    })
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> io::Result<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            child.kill()?;
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("PTY child exceeded {timeout:?}"),
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn read_master(master: &mut File) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match master.read(&mut buffer) {
            Ok(0) => return Ok(output),
            Ok(read) => output.extend_from_slice(&buffer[..read]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.raw_os_error() == Some(libc::EIO) => return Ok(output),
            Err(error) => return Err(error),
        }
    }
}
