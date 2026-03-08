use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub extension: Option<String>,
    pub size_known: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DirView {
    pub path: PathBuf,
    pub entries: Vec<DirEntry>,
}

pub struct SizeUpdate {
    pub path: PathBuf,
    pub size: u64,
}

/// Synchronous shallow scan using read_dir + metadata. ~1ms.
pub fn scan_shallow(path: &Path) -> Result<DirView, String> {
    let read = fs::read_dir(path).map_err(|e| format!("{}: {}", path.display(), e))?;

    let mut entries = Vec::new();
    for entry in read.flatten() {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.file_type().is_symlink() {
            continue;
        }

        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let extension = if is_dir {
            None
        } else {
            entry_path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
        };

        entries.push(DirEntry {
            name,
            path: entry_path,
            size,
            is_dir,
            extension,
            size_known: !is_dir,
        });
    }

    // Sort: known sizes desc (files first since dirs are 0)
    entries.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(DirView {
        path: path.to_path_buf(),
        entries,
    })
}

/// Spawn background computation of directory sizes.
/// Sends SizeUpdate for each directory entry as its recursive size is computed.
/// Returns the receiver and a cancellation token.
pub fn start_size_computation(view: &DirView) -> (mpsc::Receiver<SizeUpdate>, Arc<AtomicBool>) {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel.clone();

    let dir_entries: Vec<PathBuf> = view
        .entries
        .iter()
        .filter(|e| e.is_dir && !e.size_known)
        .map(|e| e.path.clone())
        .collect();

    std::thread::spawn(move || {
        use jwalk::WalkDir;

        dir_entries
            .into_par_iter()
            .for_each_init(
                || (tx.clone(), cancel_clone.clone()),
                |(tx, cancel), path| {
                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    let mut total: u64 = 0;
                    let mut count: u64 = 0;
                    for entry in WalkDir::new(&path)
                        .skip_hidden(false)
                        .follow_links(false)
                        .into_iter()
                        .flatten()
                    {
                        if cancel.load(Ordering::Relaxed) {
                            return;
                        }
                        if entry.file_type().is_file() {
                            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                        }
                        count += 1;
                        if count % 1000 == 0 && cancel.load(Ordering::Relaxed) {
                            return;
                        }
                    }
                    let _ = tx.send(SizeUpdate {
                        path,
                        size: total,
                    });
                },
            );
    });

    (rx, cancel)
}

// --- Disk cache ---

fn cache_dir() -> PathBuf {
    let base = dirs_next().unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("canopy-cache")
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
}

fn cache_key(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let s = canonical.to_string_lossy();
    let hash = s.bytes().fold(0u64, |h, b| {
        h.wrapping_mul(6364136223846793005).wrapping_add(b as u64)
    });
    format!("{:016x}.bin", hash)
}

pub fn load_cache(path: &Path) -> Option<DirView> {
    let cache_path = cache_dir().join(cache_key(path));
    let file = fs::File::open(&cache_path).ok()?;
    let reader = BufReader::new(file);
    bincode::deserialize_from(reader).ok()
}

pub fn save_cache(view: &DirView) {
    let dir = cache_dir();
    let _ = fs::create_dir_all(&dir);
    let cache_path = dir.join(cache_key(&view.path));
    if let Ok(file) = fs::File::create(&cache_path) {
        let writer = BufWriter::new(file);
        let _ = bincode::serialize_into(writer, view);
    }
}

pub fn cache_age(path: &Path) -> Option<std::time::Duration> {
    let cache_path = cache_dir().join(cache_key(path));
    let meta = fs::metadata(&cache_path).ok()?;
    let modified = meta.modified().ok()?;
    std::time::SystemTime::now().duration_since(modified).ok()
}

/// Returns (total_bytes, free_bytes) for the filesystem containing `path`.
pub fn disk_free_space(path: &Path) -> Option<(u64, u64)> {
    use std::ffi::CString;
    let c_path = CString::new(path.to_string_lossy().as_bytes()).ok()?;
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            let total = stat.f_blocks as u64 * stat.f_frsize as u64;
            let free = stat.f_bavail as u64 * stat.f_frsize as u64;
            Some((total, free))
        } else {
            None
        }
    }
}
