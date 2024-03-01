use clap::{crate_version, Arg, ArgAction, Command};
use fuser::{FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request, ReplyOpen, ReplyWrite};
use libc::ENOENT;
use libc::EIO;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::time::{Duration, UNIX_EPOCH};
use std::sync::atomic::{AtomicU16, AtomicU8, Ordering};
use log::error;

const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 13,
    blocks: 1,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

struct HelloFS;



impl Filesystem for HelloFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 && name.to_str() == Some("hello.txt") {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if ino == 2 {
            reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "hello.txt"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }

    fn open(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _flags: i32,
        reply: ReplyOpen
    ) {
        log::info!("LUCY open!");
        reply.opened(1, _flags as u32);
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        log::info!("LUCY write!off:{}:len:{}", _offset, _data.len());
        match OpenOptions::new()
            .write(true)
            .create(true) // Optionally create the file if it doesn't exist
            .open(format!("{}/{}", HelloFS::WRITEBACK_DIR, "abc"))
        {
            Ok(mut file) => {
                // file.seek( SeekFrom::Current(_offset));
                file.write_all(_data);
                // file.sync_all();
                reply.written(_data.len() as u32);
            },
            Err(e) => {
                error!("{}", e);
                reply.error(EIO);
            }
        }
    }

    fn mknod(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _rdev: u32,
        reply: ReplyEntry
    ) {
        log::info!("LUCY mknod!");
        match File::create( format!("{}/{}", HelloFS::WRITEBACK_DIR, "abc")) {
            Ok(mut file) => {
                let file_attr = FileAttr {
                    ino: 2,
                    size: 13,
                    blocks: 1,
                    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    kind: FileType::RegularFile,
                    perm: 0o644,
                    nlink: 1,
                    uid: 501,
                    gid: 20,
                    rdev: 0,
                    flags: 0,
                    blksize: 512,
                };
                reply.entry(&TTL, &file_attr, 0);
            },
            Err(e) => {
                error!("{}", e);
                reply.error(EIO);
            }
        }
    }
}

impl HelloFS {
    const WRITEBACK_DIR: &'static str = "/mnt-nvme2/writeback";

}

fn testfunc1() {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true) // Optionally create the file if it doesn't exist
        .open(format!("/tmp/{}", "abc"))
        .expect("Failed to open file");
    let data = String::from("LUCY-");
    file.seek(SeekFrom::Current(0));
    file.write_all(data.as_bytes());
    file.sync_all();
}

fn main() {
    // testfunc1();

    let matches = Command::new("hello")
        .version(crate_version!())
        .author("Christopher Berner")
        .arg(
            Arg::new("MOUNT_POINT")
                .required(true)
                .index(1)
                .help("Act as a client, and mount FUSE at given path"),
        )
        .arg(
            Arg::new("auto_unmount")
                .long("auto_unmount")
                .action(ArgAction::SetTrue)
                .help("Automatically unmount on process exit"),
        )
        .arg(
            Arg::new("allow-root")
                .long("allow-root")
                .action(ArgAction::SetTrue)
                .help("Allow root user to access filesystem"),
        )
        .get_matches();
    env_logger::init();
    let mountpoint = matches.get_one::<String>("MOUNT_POINT").unwrap();
    let mut options = vec![MountOption::RW, MountOption::FSName("hello".to_string())];
    if matches.get_flag("auto_unmount") {
        options.push(MountOption::AutoUnmount);
    }
    if matches.get_flag("allow-root") {
        options.push(MountOption::AllowRoot);
    }
    fuser::mount2(HelloFS, mountpoint, &options).unwrap();
}
