//! The content-addressed artifact store and the artifact manifest (engine-spec §8.11 "Artifact
//! write protocol"; §14.3; DR-C15; work package P1-02 D16).
//!
//! **Layout.** Under the data directory (default `data/`, engine-spec §8.5):
//!
//! - `artifacts/sha256/<h[0..2]>/<h>`: an immutable file whose name is the SHA-256 `h` of its
//!   bytes. The path is always derived from the hash ([`relative_path`]), never read from a
//!   stored string, so a hash can only name a path made of `0-9a-f` inside the store.
//! - `artifacts/tmp/<pid>-<nanos>-<counter>.part`: a file being written, on the same volume.
//!
//! **The §8.11 order.** [`ArtifactStore::put_with`]:
//!
//! 1. writes the producer's bytes to a new temporary file (`create_new`), hashing and counting
//!    them on the way;
//! 2. flushes and `sync_all`s it, then closes the handle (Windows renames no open file);
//! 3. derives the final path from the digest; if a file is already there it is re-verified:
//!    equal content is de-duplicated (the temporary file is removed), different content is
//!    `StateCorrupt` and is never overwritten;
//! 4. otherwise renames the temporary file into place and, on Unix, fsyncs the directory. Windows
//!    cannot fsync a directory from `std`; that gap is recorded for P1-11 (D16, F-4).
//!
//! The result is a [`StagedArtifact`]: durable, not yet registered. [`register_in`] records the
//! sha256 in `artifact_manifest` inside the caller's write transaction (step 4), and the caller's
//! commit is step 5. A file with no manifest row is garbage ([`ArtifactStore::collect_garbage`]);
//! a manifest row whose file is missing, short or altered is `StateCorrupt`, never empty state
//! ([`ArtifactStore::read_verified`]; KI-A8, generic part).
//!
//! The file functions are synchronous `std::fs` (a computation stage calls them on a blocking
//! thread); the manifest functions are async SQL.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use grid_domain::{JobId, UtcInstant};

use crate::codec::{id_from_sql, id_to_sql, instant_from_sql, instant_to_sql};
use crate::db::{Database, WriteTx};
use crate::error::PersistenceError;
use crate::hash::{Sha256Hasher, Sha256Hex};

/// The store's directory under the data directory.
pub const ARTIFACTS_DIR: &str = "artifacts";
/// The content-addressed subdirectory of [`ARTIFACTS_DIR`].
pub const SHA256_DIR: &str = "sha256";
/// The temporary-file subdirectory of [`ARTIFACTS_DIR`].
pub const TMP_DIR: &str = "tmp";
/// The extension of a temporary file.
pub const TMP_EXTENSION: &str = "part";

/// The longest `format` and `schema_version` label, in characters (the manifest CHECKs).
const MAX_LABEL_CHARS: usize = 64;
/// How many temporary names `put_with` tries before it gives up.
const TEMP_NAME_ATTEMPTS: u32 = 16;
/// How many times a rename or removal is retried on Windows `PermissionDenied`.
const WINDOWS_RETRIES: u32 = 10;
/// The read buffer of the verifying hash.
const READ_CHUNK: usize = 64 * 1024;

/// A process-wide counter that keeps temporary names unique within one process.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A registered artifact's manifest row id (`artifact_manifest.artifact_id`; work package P1-02
/// D4). Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactId(pub u64);

impl fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The labels of an artifact: its format and schema version (engine-spec §8.5
/// `artifact_manifest`). Validated on construction against the manifest CHECKs, so a
/// registration never fails on them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactMeta {
    format: String,
    schema_version: String,
}

impl ArtifactMeta {
    /// Builds the labels.
    ///
    /// - `format`: 1 to 64 characters of `a-z`, `0-9`, `_`, `.` and `-`, for example `parquet`
    ///   or `f64-le`.
    /// - `schema_version`: 1 to 64 characters of `A-Z`, `a-z`, `0-9`, `_`, `.`, `+` and `-`.
    ///
    /// # Errors
    ///
    /// [`PersistenceError::InvalidInput`] naming the label that fails.
    pub fn new(
        format: impl Into<String>,
        schema_version: impl Into<String>,
    ) -> Result<Self, PersistenceError> {
        let format = format.into();
        let schema_version = schema_version.into();
        check_label("artifact format", &format, |c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-')
        })?;
        check_label("artifact schema version", &schema_version, |c| {
            c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '+' | '-')
        })?;
        Ok(Self {
            format,
            schema_version,
        })
    }

    /// The format label.
    pub fn format(&self) -> &str {
        &self.format
    }

    /// The schema-version label.
    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }
}

/// Checks a label's length and alphabet.
fn check_label(
    what: &'static str,
    label: &str,
    allowed: impl Fn(char) -> bool,
) -> Result<(), PersistenceError> {
    let length = label.chars().count();
    if length == 0 || length > MAX_LABEL_CHARS {
        return Err(PersistenceError::invalid_input(
            what,
            format!("must be 1 to {MAX_LABEL_CHARS} characters; got {length}"),
        ));
    }
    if let Some(bad) = label.chars().find(|c| !allowed(*c)) {
        return Err(PersistenceError::invalid_input(
            what,
            format!("character {bad:?} is not allowed"),
        ));
    }
    Ok(())
}

/// An artifact durably written to its content address but not registered yet (engine-spec §8.11
/// steps 1–3). Only [`ArtifactStore`] creates one, so a value always names a file that was
/// written and synced.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StagedArtifact {
    sha256: Sha256Hex,
    byte_size: u64,
    meta: ArtifactMeta,
    deduplicated: bool,
}

impl StagedArtifact {
    /// The content hash.
    pub const fn sha256(&self) -> &Sha256Hex {
        &self.sha256
    }

    /// The size in bytes.
    pub const fn byte_size(&self) -> u64 {
        self.byte_size
    }

    /// The labels.
    pub const fn meta(&self) -> &ArtifactMeta {
        &self.meta
    }

    /// Whether identical content was already at the content address, so this put wrote no new
    /// file.
    pub const fn deduplicated(&self) -> bool {
        self.deduplicated
    }

    /// The path relative to the data directory ([`relative_path`]).
    pub fn relative_path(&self) -> String {
        relative_path(&self.sha256)
    }
}

/// One row of `artifact_manifest` (engine-spec §8.5: path, sha256, byte size, format, schema
/// version, producing job, `created_at`). Immutable once written. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManifestEntry {
    /// The row id. Identical content registered twice is two rows (two lineages) and one file.
    pub artifact_id: ArtifactId,
    /// The content hash; the path is derived from it.
    pub sha256: Sha256Hex,
    /// The size in bytes.
    pub byte_size: u64,
    /// The labels.
    pub meta: ArtifactMeta,
    /// The job whose transaction registered the artifact.
    pub producing_job: JobId,
    /// When the row was written, in whole seconds (engine-spec §4.5).
    pub created_at: UtcInstant,
}

impl ManifestEntry {
    /// The path relative to the data directory ([`relative_path`]).
    pub fn relative_path(&self) -> String {
        relative_path(&self.sha256)
    }
}

/// The path of a content-addressed file relative to the data directory, with `/` separators:
/// `artifacts/sha256/<h[0..2]>/<h>`, the value the manifest's `path` CHECK requires (work
/// package P1-02 D16).
pub fn relative_path(sha256: &Sha256Hex) -> String {
    format!(
        "{ARTIFACTS_DIR}/{SHA256_DIR}/{}/{}",
        sha256.shard(),
        sha256.to_hex()
    )
}

/// A failure of [`ArtifactStore::put_with`]: the producer's own error, or the store's.
#[derive(Debug)]
pub enum PutError<E> {
    /// The producer returned an error. No final file was written, and the temporary file was
    /// removed.
    Producer(E),
    /// The store failed: I/O, or an existing file at the content address with other content
    /// (`StateCorrupt`). No final file was written by this put.
    Store(PersistenceError),
}

impl<E: fmt::Display> fmt::Display for PutError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PutError::Producer(error) => write!(f, "the artifact producer failed: {error}"),
            PutError::Store(error) => write!(f, "{error}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for PutError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PutError::Producer(error) => Some(error),
            PutError::Store(error) => Some(error),
        }
    }
}

/// What [`ArtifactStore::collect_garbage`] did. Every path is relative to the data directory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct GcReport {
    /// Temporary `*.part` files older than the threshold that were removed, sorted.
    pub removed_temp: Vec<String>,
    /// Well-named content-addressed files absent from the registered set and older than the
    /// threshold that were removed, sorted.
    pub removed_unregistered: Vec<Sha256Hex>,
    /// Files and directories whose names are not the store's, sorted. They are reported, never
    /// removed.
    pub unknown: Vec<String>,
    /// How many collectable files were kept because they are younger than the threshold.
    pub kept_recent: u64,
}

/// The content-addressed artifact store under one data directory (work package P1-02 D16).
///
/// Cheap to clone; it holds only the data directory. Synchronous `std::fs`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    /// A store under `data_dir`. Nothing is created until the first put.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            root: data_dir.into(),
        }
    }

    /// The data directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The absolute (or root-relative) path of a content-addressed file, built from the hash
    /// with [`Path::join`].
    fn final_path(&self, sha256: &Sha256Hex) -> PathBuf {
        self.shard_dir(sha256).join(sha256.to_hex())
    }

    /// The shard directory of a hash.
    fn shard_dir(&self, sha256: &Sha256Hex) -> PathBuf {
        self.sha256_dir().join(sha256.shard())
    }

    /// `artifacts/sha256`.
    fn sha256_dir(&self) -> PathBuf {
        self.root.join(ARTIFACTS_DIR).join(SHA256_DIR)
    }

    /// `artifacts/tmp`.
    fn tmp_dir(&self) -> PathBuf {
        self.root.join(ARTIFACTS_DIR).join(TMP_DIR)
    }

    /// Writes an artifact by running `producer` against a writer (engine-spec §8.11 steps 1–3;
    /// the module documentation lists the steps).
    ///
    /// The producer receives a buffered writer to the temporary file. Its bytes are hashed and
    /// counted as they reach the file. When the producer returns `Ok`, the file is flushed,
    /// synced, closed and moved to its content address.
    ///
    /// # Errors
    ///
    /// - [`PutError::Producer`] with the producer's error; the temporary file is removed and no
    ///   final file is written.
    /// - [`PutError::Store`] with [`PersistenceError::Io`] when the store's own I/O fails,
    ///   including a write the producer saw fail (even if the producer then returned `Ok`), so a
    ///   truncated file is never moved into place.
    /// - [`PutError::Store`] with [`PersistenceError::StateCorrupt`] when a file with other
    ///   content already sits at the content address; it is left untouched.
    pub fn put_with<E, F>(
        &self,
        meta: ArtifactMeta,
        producer: F,
    ) -> Result<StagedArtifact, PutError<E>>
    where
        F: FnOnce(&mut dyn Write) -> Result<(), E>,
    {
        let tmp_dir = self.tmp_dir();
        fs::create_dir_all(&tmp_dir).map_err(|error| {
            PutError::Store(PersistenceError::io(
                "creating the artifact temporary directory",
                &error,
                Some(format!("{ARTIFACTS_DIR}/{TMP_DIR}")),
            ))
        })?;
        let (temp_path, temp_shown, file) = self.create_temp().map_err(PutError::Store)?;

        let mut writer = HashingWriter {
            inner: BufWriter::new(file),
            hasher: Sha256Hasher::new(),
            failed: None,
        };
        let produced = producer(&mut writer);
        let HashingWriter {
            inner,
            hasher,
            failed,
        } = writer;

        let synced = match (produced, failed) {
            (Err(error), None) => {
                drop(inner);
                remove_best_effort(&temp_path);
                return Err(PutError::Producer(error));
            }
            (_, Some(kind)) => {
                drop(inner);
                remove_best_effort(&temp_path);
                return Err(PutError::Store(PersistenceError::Io {
                    context: "writing an artifact to its temporary file",
                    kind,
                    path: Some(temp_shown),
                }));
            }
            (Ok(()), None) => finish_temp(inner, &temp_shown),
        };
        if let Err(error) = synced {
            remove_best_effort(&temp_path);
            return Err(PutError::Store(error));
        }

        let (sha256, byte_size) = hasher.finalize();
        match self.move_into_place(&temp_path, &temp_shown, &sha256, byte_size) {
            Ok(deduplicated) => Ok(StagedArtifact {
                sha256,
                byte_size,
                meta,
                deduplicated,
            }),
            Err(error) => {
                remove_best_effort(&temp_path);
                Err(PutError::Store(error))
            }
        }
    }

    /// Writes `bytes` as an artifact ([`ArtifactStore::put_with`] with a producer that writes
    /// them).
    ///
    /// # Errors
    ///
    /// As [`ArtifactStore::put_with`], with every failure as a [`PersistenceError`].
    pub fn put_bytes(
        &self,
        meta: ArtifactMeta,
        bytes: &[u8],
    ) -> Result<StagedArtifact, PersistenceError> {
        self.put_with(meta, |writer| writer.write_all(bytes))
            .map_err(|error| match error {
                PutError::Store(error) => error,
                PutError::Producer(error) => {
                    PersistenceError::io("writing an artifact to its temporary file", &error, None)
                }
            })
    }

    /// Creates a new temporary file with `create_new`, retrying on a name collision.
    fn create_temp(&self) -> Result<(PathBuf, String, File), PersistenceError> {
        let pid = std::process::id();
        let mut last_kind = io::ErrorKind::AlreadyExists;
        for _ in 0..TEMP_NAME_ATTEMPTS {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos());
            let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let name = format!("{pid}-{nanos}-{counter}.{TMP_EXTENSION}");
            let path = self.tmp_dir().join(&name);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok((path, format!("{ARTIFACTS_DIR}/{TMP_DIR}/{name}"), file));
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    last_kind = error.kind();
                }
                Err(error) => {
                    return Err(PersistenceError::io(
                        "creating an artifact temporary file",
                        &error,
                        Some(format!("{ARTIFACTS_DIR}/{TMP_DIR}/{name}")),
                    ));
                }
            }
        }
        Err(PersistenceError::Io {
            context: "choosing a unique artifact temporary file name",
            kind: last_kind,
            path: Some(format!("{ARTIFACTS_DIR}/{TMP_DIR}")),
        })
    }

    /// Moves a synced temporary file to its content address, or de-duplicates it against an
    /// equal file already there. Returns whether it de-duplicated.
    fn move_into_place(
        &self,
        temp_path: &Path,
        temp_shown: &str,
        sha256: &Sha256Hex,
        byte_size: u64,
    ) -> Result<bool, PersistenceError> {
        let sha256_dir = self.sha256_dir();
        let shard_dir = self.shard_dir(sha256);
        let shown = relative_path(sha256);
        let shard_is_new = !shard_dir.is_dir();
        fs::create_dir_all(&shard_dir).map_err(|error| {
            PersistenceError::io(
                "creating an artifact shard directory",
                &error,
                Some(format!("{ARTIFACTS_DIR}/{SHA256_DIR}/{}", sha256.shard())),
            )
        })?;
        if shard_is_new {
            sync_dir(&sha256_dir).map_err(|error| {
                PersistenceError::io(
                    "syncing the artifact directory",
                    &error,
                    Some(format!("{ARTIFACTS_DIR}/{SHA256_DIR}")),
                )
            })?;
        }

        let final_path = self.final_path(sha256);
        match fs::symlink_metadata(&final_path) {
            Ok(_) => {
                // Never overwrite: re-verify what is there. Equal content de-duplicates.
                self.verify_file(sha256, byte_size)?;
                remove_with_retry(temp_path).map_err(|error| {
                    PersistenceError::io(
                        "removing a de-duplicated temporary file",
                        &error,
                        Some(temp_shown.to_owned()),
                    )
                })?;
                Ok(true)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                rename_with_retry(temp_path, &final_path).map_err(|error| {
                    PersistenceError::io(
                        "renaming an artifact into place",
                        &error,
                        Some(shown.clone()),
                    )
                })?;
                sync_dir(&shard_dir).map_err(|error| {
                    PersistenceError::io("syncing an artifact shard directory", &error, Some(shown))
                })?;
                Ok(false)
            }
            Err(error) => Err(PersistenceError::io(
                "checking an artifact's content address",
                &error,
                Some(shown),
            )),
        }
    }

    /// Checks the content-addressed file of `sha256`: it exists, is a regular file, has
    /// `byte_size` bytes and hashes to `sha256` (engine-spec §8.11; §14.3 "the manifest hash
    /// verified on read"). The path is derived from the hash.
    ///
    /// # Errors
    ///
    /// [`PersistenceError::StateCorrupt`] when the file is missing, not a regular file, of
    /// another size or of other content; [`PersistenceError::Io`] when it cannot be read.
    pub fn verify_file(&self, sha256: &Sha256Hex, byte_size: u64) -> Result<(), PersistenceError> {
        self.read_checked(sha256, byte_size, false).map(|_| ())
    }

    /// Reads a registered artifact and verifies it against its manifest row (engine-spec §8.11:
    /// "a manifest row whose file is missing, or whose file hash does not match, is the typed
    /// failure `StateCorrupt`. It is never treated as an empty state"; KI-A8, generic part).
    ///
    /// # Errors
    ///
    /// As [`ArtifactStore::verify_file`].
    pub fn read_verified(&self, entry: &ManifestEntry) -> Result<Vec<u8>, PersistenceError> {
        self.read_checked(&entry.sha256, entry.byte_size, true)
    }

    /// Streams the content-addressed file of `sha256` through the hash, keeping the bytes when
    /// `keep` is set.
    fn read_checked(
        &self,
        sha256: &Sha256Hex,
        byte_size: u64,
        keep: bool,
    ) -> Result<Vec<u8>, PersistenceError> {
        let shown = relative_path(sha256);
        let path = self.final_path(sha256);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(PersistenceError::state_corrupt(format!(
                    "registered artifact `{shown}` is missing"
                )));
            }
            Err(error) => {
                return Err(PersistenceError::io(
                    "reading an artifact's metadata",
                    &error,
                    Some(shown),
                ));
            }
        };
        if !metadata.is_file() {
            return Err(PersistenceError::state_corrupt(format!(
                "artifact `{shown}` is not a regular file"
            )));
        }
        if metadata.len() != byte_size {
            return Err(PersistenceError::state_corrupt(format!(
                "artifact `{shown}` has {} bytes; its manifest records {byte_size}",
                metadata.len()
            )));
        }

        let mut file = File::open(&path).map_err(|error| {
            PersistenceError::io("opening an artifact", &error, Some(shown.clone()))
        })?;
        let mut hasher = Sha256Hasher::new();
        let mut bytes = Vec::new();
        let mut chunk = vec![0u8; READ_CHUNK];
        loop {
            let read = match file.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    return Err(PersistenceError::io(
                        "reading an artifact",
                        &error,
                        Some(shown),
                    ));
                }
            };
            hasher.update(&chunk[..read]);
            if keep {
                bytes.extend_from_slice(&chunk[..read]);
            }
        }
        let (found, length) = hasher.finalize();
        if length != byte_size {
            return Err(PersistenceError::state_corrupt(format!(
                "artifact `{shown}` changed size while it was read ({length} bytes; its manifest \
                 records {byte_size})"
            )));
        }
        if found != *sha256 {
            return Err(PersistenceError::state_corrupt(format!(
                "artifact `{shown}` hashes to {found}, not to its name"
            )));
        }
        Ok(bytes)
    }

    /// Removes every temporary `*.part` file under `artifacts/tmp/`, whatever its age, and
    /// returns how many it removed. Other names there are left untouched.
    ///
    /// **Precondition:** no other process is writing to this store. Under the lock model of owner
    /// ruling R5 every invocation holds the store lock shared, so only a holder of the exclusive
    /// lock (`grid migrate`) may call this; a concurrent writer's in-progress file would
    /// otherwise be removed under it. [`ArtifactStore::collect_garbage`] is the age-bounded form
    /// that is safe alongside other writers.
    ///
    /// # Errors
    ///
    /// [`PersistenceError::Io`] when the directory cannot be listed or a file cannot be removed.
    pub fn purge_temp(&self) -> Result<u64, PersistenceError> {
        let mut removed = 0;
        for (name, path, file_type) in
            list_dir(&self.tmp_dir(), &format!("{ARTIFACTS_DIR}/{TMP_DIR}"))?
        {
            if file_type.is_file() && is_temp_name(&name) {
                remove_with_retry(&path).map_err(|error| {
                    PersistenceError::io(
                        "removing an orphaned temporary file",
                        &error,
                        Some(format!("{ARTIFACTS_DIR}/{TMP_DIR}/{name}")),
                    )
                })?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Removes garbage older than `older_than` and reports what it did (engine-spec §8.11: "an
    /// artifact file with no manifest row is garbage-collectable"; work package P1-02 D16).
    ///
    /// It removes only:
    ///
    /// - temporary `*.part` files under `artifacts/tmp/`;
    /// - content-addressed files `artifacts/sha256/<h[0..2]>/<h>` whose name is a well-formed
    ///   hash, whose shard matches it, and whose hash is not in `registered`;
    ///
    /// and only when the file's modification time is at least `older_than` in the past (a file
    /// whose age cannot be read is kept). Every other name is reported in [`GcReport::unknown`]
    /// and never removed. `registered` is the manifest's hash set ([`registered_hashes`]), read
    /// before the call; a file registered after that read is younger than any sensible threshold.
    /// Scheduling is P1-11's (F-4).
    ///
    /// # Errors
    ///
    /// [`PersistenceError::Io`] when a directory cannot be listed or a file cannot be removed.
    pub fn collect_garbage(
        &self,
        registered: &BTreeSet<Sha256Hex>,
        older_than: Duration,
    ) -> Result<GcReport, PersistenceError> {
        let now = SystemTime::now();
        let is_old = |path: &Path| -> bool {
            fs::symlink_metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age >= older_than)
        };
        let mut report = GcReport::default();

        let tmp_shown = format!("{ARTIFACTS_DIR}/{TMP_DIR}");
        for (name, path, file_type) in list_dir(&self.tmp_dir(), &tmp_shown)? {
            let shown = format!("{tmp_shown}/{name}");
            if !(file_type.is_file() && is_temp_name(&name)) {
                report.unknown.push(shown);
            } else if is_old(&path) {
                remove_with_retry(&path).map_err(|error| {
                    PersistenceError::io(
                        "removing a stale temporary file",
                        &error,
                        Some(shown.clone()),
                    )
                })?;
                report.removed_temp.push(shown);
            } else {
                report.kept_recent += 1;
            }
        }

        let sha_shown = format!("{ARTIFACTS_DIR}/{SHA256_DIR}");
        for (shard, shard_path, shard_type) in list_dir(&self.sha256_dir(), &sha_shown)? {
            let shard_shown = format!("{sha_shown}/{shard}");
            if !(shard_type.is_dir() && is_shard_name(&shard)) {
                report.unknown.push(shard_shown);
                continue;
            }
            for (name, path, file_type) in list_dir(&shard_path, &shard_shown)? {
                let shown = format!("{shard_shown}/{name}");
                let hash = name
                    .parse::<Sha256Hex>()
                    .ok()
                    .filter(|hash| file_type.is_file() && hash.shard() == shard);
                match hash {
                    None => report.unknown.push(shown),
                    Some(hash) if registered.contains(&hash) => {}
                    Some(hash) if is_old(&path) => {
                        remove_with_retry(&path).map_err(|error| {
                            PersistenceError::io(
                                "removing an unregistered artifact",
                                &error,
                                Some(shown.clone()),
                            )
                        })?;
                        report.removed_unregistered.push(hash);
                    }
                    Some(_) => report.kept_recent += 1,
                }
            }
        }

        report.removed_temp.sort();
        report.removed_unregistered.sort();
        report.unknown.sort();
        Ok(report)
    }
}

/// A writer that hashes and counts the bytes that reach the inner writer, and remembers the first
/// write failure so a producer that ignores it cannot get a truncated file moved into place.
struct HashingWriter {
    inner: BufWriter<File>,
    hasher: Sha256Hasher,
    failed: Option<io::ErrorKind>,
}

impl Write for HashingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(kind) = self.failed {
            return Err(io::Error::from(kind));
        }
        match self.inner.write(buf) {
            Ok(written) => {
                self.hasher.update(&buf[..written]);
                Ok(written)
            }
            Err(error) => {
                if error.kind() != io::ErrorKind::Interrupted {
                    self.failed = Some(error.kind());
                }
                Err(error)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(kind) = self.failed {
            return Err(io::Error::from(kind));
        }
        self.inner.flush().inspect_err(|error| {
            self.failed = Some(error.kind());
        })
    }
}

/// Flushes, syncs and closes a temporary file (engine-spec §8.11 step 2).
fn finish_temp(writer: BufWriter<File>, shown: &str) -> Result<(), PersistenceError> {
    let file = writer.into_inner().map_err(|error| {
        PersistenceError::io(
            "flushing an artifact temporary file",
            error.error(),
            Some(shown.to_owned()),
        )
    })?;
    file.sync_all().map_err(|error| {
        PersistenceError::io(
            "syncing an artifact temporary file",
            &error,
            Some(shown.to_owned()),
        )
    })?;
    // The handle closes here, before the rename: Windows renames no open file.
    drop(file);
    Ok(())
}

/// Whether `name` is a temporary-file name: anything ending in `.part`.
fn is_temp_name(name: &str) -> bool {
    name.len() > TMP_EXTENSION.len() + 1
        && name.ends_with(TMP_EXTENSION)
        && name.as_bytes()[name.len() - TMP_EXTENSION.len() - 1] == b'.'
}

/// Whether `name` is a shard directory name: two lowercase hexadecimal characters.
fn is_shard_name(name: &str) -> bool {
    name.len() == 2
        && name
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Lists a directory as sorted `(name, path, type)` triples; a missing directory is empty. A name
/// that is not valid UTF-8 is listed under its lossy form, which never parses as a hash, so it is
/// reported as unknown and never removed.
fn list_dir(
    dir: &Path,
    shown: &str,
) -> Result<Vec<(String, PathBuf, fs::FileType)>, PersistenceError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(PersistenceError::io(
                "listing an artifact directory",
                &error,
                Some(shown.to_owned()),
            ));
        }
    };
    let mut listed = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            PersistenceError::io(
                "listing an artifact directory",
                &error,
                Some(shown.to_owned()),
            )
        })?;
        let file_type = entry.file_type().map_err(|error| {
            PersistenceError::io(
                "reading an artifact directory entry",
                &error,
                Some(shown.to_owned()),
            )
        })?;
        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(raw) => format!("{}\u{fffd}", raw.to_string_lossy()),
        };
        listed.push((name, entry.path(), file_type));
    }
    listed.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(listed)
}

/// Fsyncs a directory so a new entry in it survives a power loss (Unix).
#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// Windows cannot fsync a directory from `std`; the gap is recorded for P1-11 (work package
/// P1-02 D16, F-4). A rename lost to a power loss is detected on read as `StateCorrupt`.
#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}

/// Runs `op`, retrying a bounded number of times on Windows `PermissionDenied` (a virus scanner
/// or indexer holding the file briefly; work package P1-02 "Platforms"). Elsewhere it runs once.
fn retry_permission_denied<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut attempt = 1;
    loop {
        match op() {
            Err(error)
                if cfg!(windows)
                    && error.kind() == io::ErrorKind::PermissionDenied
                    && attempt < WINDOWS_RETRIES =>
            {
                std::thread::sleep(Duration::from_millis(25 * u64::from(attempt)));
                attempt += 1;
            }
            result => return result,
        }
    }
}

/// `fs::rename` with the Windows retry.
fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    retry_permission_denied(|| fs::rename(from, to))
}

/// `fs::remove_file` with the Windows retry.
fn remove_with_retry(path: &Path) -> io::Result<()> {
    retry_permission_denied(|| fs::remove_file(path))
}

/// Removes a temporary file on a failure path, ignoring a failure to do so: the file is then an
/// orphan that [`ArtifactStore::collect_garbage`] removes later.
fn remove_best_effort(path: &Path) {
    // The failure being reported is the caller's; an orphaned temporary file is collectable.
    let _ignored = remove_with_retry(path);
}

/// Records a staged artifact in `artifact_manifest` inside the caller's write transaction
/// (engine-spec §8.11 step 4; the caller's commit is step 5).
///
/// A hash may be registered more than once (two lineages, one file), always with one size.
///
/// # Errors
///
/// - [`PersistenceError::StateCorrupt`] when the hash is already registered with another size;
/// - [`PersistenceError::IdOutOfRange`] when an id does not fit;
/// - [`PersistenceError::Constraint`] when the store refuses the row (an unknown producing job);
/// - [`PersistenceError::Database`] or [`PersistenceError::Busy`] for SQLite failures.
pub async fn register_in(
    tx: &mut WriteTx,
    staged: &StagedArtifact,
    producing_job: JobId,
    now: UtcInstant,
) -> Result<ManifestEntry, PersistenceError> {
    let sha256 = staged.sha256.to_hex();
    let path = relative_path(&staged.sha256);
    let byte_size =
        i64::try_from(staged.byte_size).map_err(|_| PersistenceError::IdOutOfRange {
            what: "artifact byte size",
            value: staged.byte_size,
        })?;
    let job = id_to_sql("producing job id", producing_job.0)?;
    let created_at = instant_to_sql(now);

    let conflict = sqlx::query!(
        r#"SELECT byte_size AS "byte_size!: i64" FROM artifact_manifest
           WHERE sha256 = ? AND byte_size <> ? LIMIT 1"#,
        sha256,
        byte_size
    )
    .fetch_optional(tx.connection())
    .await
    .map_err(|error| {
        PersistenceError::from_sqlx("checking an artifact's registered size", error)
    })?;
    if let Some(row) = conflict {
        return Err(PersistenceError::state_corrupt(format!(
            "artifact {sha256} is registered with {} bytes; this registration has {byte_size}",
            row.byte_size
        )));
    }

    let format = staged.meta.format();
    let schema_version = staged.meta.schema_version();
    let inserted = sqlx::query!(
        "INSERT INTO artifact_manifest \
         (sha256, path, byte_size, format, schema_version, producing_job_id, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        sha256,
        path,
        byte_size,
        format,
        schema_version,
        job,
        created_at
    )
    .execute(tx.connection())
    .await
    .map_err(|error| PersistenceError::from_sqlx("registering an artifact", error))?;

    let artifact_id = id_from_sql("artifact_id", inserted.last_insert_rowid())?;
    Ok(ManifestEntry {
        artifact_id: ArtifactId(artifact_id),
        sha256: staged.sha256,
        byte_size: staged.byte_size,
        meta: staged.meta.clone(),
        producing_job,
        created_at: now,
    })
}

/// A manifest row as stored, before it is checked.
struct StoredEntry {
    artifact_id: i64,
    sha256: String,
    path: String,
    byte_size: i64,
    format: String,
    schema_version: String,
    producing_job_id: i64,
    created_at: i64,
}

impl StoredEntry {
    /// Checks a stored row and converts it (DR-B6: a malformed row is `StateCorrupt`).
    fn into_entry(self) -> Result<ManifestEntry, PersistenceError> {
        let sha256 = self.sha256.parse::<Sha256Hex>().map_err(|error| {
            PersistenceError::state_corrupt(format!(
                "artifact_manifest row {} holds a malformed sha256: {error}",
                self.artifact_id
            ))
        })?;
        if self.path != relative_path(&sha256) {
            return Err(PersistenceError::state_corrupt(format!(
                "artifact_manifest row {} holds a path that is not derived from its hash",
                self.artifact_id
            )));
        }
        let meta = ArtifactMeta::new(self.format, self.schema_version).map_err(|error| {
            PersistenceError::state_corrupt(format!(
                "artifact_manifest row {} holds invalid labels: {error}",
                self.artifact_id
            ))
        })?;
        Ok(ManifestEntry {
            artifact_id: ArtifactId(id_from_sql("artifact_id", self.artifact_id)?),
            sha256,
            byte_size: id_from_sql("artifact byte_size", self.byte_size)?,
            meta,
            producing_job: JobId(id_from_sql("producing_job_id", self.producing_job_id)?),
            created_at: instant_from_sql(self.created_at),
        })
    }
}

/// The manifest row `artifact_id`, or `None` when no row has that id.
///
/// # Errors
///
/// [`PersistenceError::StateCorrupt`] when the row is malformed; [`PersistenceError::Database`]
/// for SQLite failures.
pub async fn entry(
    db: &Database,
    artifact_id: ArtifactId,
) -> Result<Option<ManifestEntry>, PersistenceError> {
    let id = id_to_sql("artifact_id", artifact_id.0)?;
    let row = sqlx::query_as!(
        StoredEntry,
        r#"SELECT artifact_id AS "artifact_id!: i64", sha256, path, byte_size, format,
                  schema_version, producing_job_id, created_at
           FROM artifact_manifest WHERE artifact_id = ?"#,
        id
    )
    .fetch_optional(db.pool())
    .await
    .map_err(|error| PersistenceError::from_sqlx("reading an artifact manifest row", error))?;
    row.map(StoredEntry::into_entry).transpose()
}

/// Every manifest row of `sha256`, in `artifact_id` order (one per registration).
///
/// # Errors
///
/// As [`entry`].
pub async fn entries_for(
    db: &Database,
    sha256: &Sha256Hex,
) -> Result<Vec<ManifestEntry>, PersistenceError> {
    let hex = sha256.to_hex();
    let rows = sqlx::query_as!(
        StoredEntry,
        r#"SELECT artifact_id AS "artifact_id!: i64", sha256, path, byte_size, format,
                  schema_version, producing_job_id, created_at
           FROM artifact_manifest WHERE sha256 = ? ORDER BY artifact_id"#,
        hex
    )
    .fetch_all(db.pool())
    .await
    .map_err(|error| PersistenceError::from_sqlx("reading artifact manifest rows", error))?;
    rows.into_iter().map(StoredEntry::into_entry).collect()
}

/// The set of registered hashes, the input of [`ArtifactStore::collect_garbage`].
///
/// # Errors
///
/// [`PersistenceError::StateCorrupt`] when a stored hash is malformed;
/// [`PersistenceError::Database`] for SQLite failures.
pub async fn registered_hashes(db: &Database) -> Result<BTreeSet<Sha256Hex>, PersistenceError> {
    let rows = sqlx::query!("SELECT DISTINCT sha256 FROM artifact_manifest ORDER BY sha256")
        .fetch_all(db.pool())
        .await
        .map_err(|error| PersistenceError::from_sqlx("reading the registered hashes", error))?;
    rows.into_iter()
        .map(|row| {
            row.sha256.parse::<Sha256Hex>().map_err(|error| {
                PersistenceError::state_corrupt(format!(
                    "artifact_manifest holds a malformed sha256: {error}"
                ))
            })
        })
        .collect()
}
