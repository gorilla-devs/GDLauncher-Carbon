//! Reads a Minecraft region file (`r.<x>.<z>.mca`): the whole file is read into
//! memory and closed at once, then the inflated bytes of one chunk are handed out
//! at a time. Nothing in the file is trusted: every offset and length is checked
//! against the file's real size, and inflating is capped.

use std::io::Read;
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::{Decompress, FlushDecompress, Status};

const SECTOR: usize = 4096;
const HEADER: usize = 2 * SECTOR;

/// A region file above this size is left empty.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) const MAX_REGION_BYTES: u64 = 256 * 1024 * 1024;
/// A chunk inflating past this size is left empty.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) const MAX_CHUNK_BYTES: usize = 32 * 1024 * 1024;

#[cfg_attr(not(test), allow(dead_code))]
pub(super) struct Region {
    bytes: Vec<u8>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Region {
    /// `None` for a missing file, a link, anything but a regular file, a file shorter
    /// than the two header sectors or one above `MAX_REGION_BYTES`.
    pub(super) fn read(path: &Path) -> std::io::Result<Option<Region>> {
        let meta = match std::fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        if !meta.is_file() || meta.len() < HEADER as u64 || meta.len() > MAX_REGION_BYTES {
            return Ok(None);
        }
        // The file may grow while it is read (the game is running): the cap holds
        // for what is actually read, not only for what the metadata said.
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            // a link swapped in after the check above is refused, not followed
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
        let file = match options.open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            #[cfg(unix)]
            Err(e) if e.raw_os_error() == Some(libc::ELOOP) => return Ok(None),
            Err(e) => return Err(e),
        };
        // what was opened is checked again: it must be the regular file it should be
        let opened = file.metadata()?;
        if !opened.is_file() || opened.len() < HEADER as u64 || opened.len() > MAX_REGION_BYTES {
            return Ok(None);
        }
        let mut bytes = Vec::with_capacity(opened.len() as usize);
        file.take(MAX_REGION_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() < HEADER || bytes.len() as u64 > MAX_REGION_BYTES {
            return Ok(None);
        }
        Ok(Some(Region { bytes }))
    }

    /// The location entry of a chunk slot: `(sector offset, sector count)`, when
    /// the slot is in range and holds a chunk.
    fn entry(&self, x: u8, z: u8) -> Option<(usize, usize)> {
        if x >= 32 || z >= 32 {
            return None;
        }
        let at = (x as usize + z as usize * 32) * 4;
        let e = self.bytes.get(at..at + 4)?;
        let offset = u32::from_be_bytes([0, e[0], e[1], e[2]]) as usize;
        let count = e[3] as usize;
        if offset == 0 && count == 0 {
            return None;
        }
        Some((offset, count))
    }

    /// The compression kind and compressed bytes of a chunk, after every bound is checked.
    fn payload(&self, x: u8, z: u8) -> Option<(u8, &[u8])> {
        let (offset, count) = self.entry(x, z)?;
        if offset < 2 || count == 0 {
            return None;
        }
        let start = offset.checked_mul(SECTOR)?;
        let head = self.bytes.get(start..start.checked_add(5)?)?;
        let length = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        if length == 0 || length > count.checked_mul(SECTOR)? {
            return None;
        }
        let end = start.checked_add(4)?.checked_add(length)?;
        let data = self.bytes.get(start + 5..end)?;
        Some((head[4], data))
    }

    /// Whether the table lists a chunk here with bounds that hold; not whether
    /// `chunk` can read it, as its stream may still be broken or too large.
    pub(super) fn has_chunk(&self, x: u8, z: u8) -> bool {
        self.payload(x, z).is_some()
    }

    /// The inflated chunk NBT. `None` for an empty or lying slot, LZ4, an external
    /// file, an unknown kind or a stream that is broken or too large.
    pub(super) fn chunk(&self, x: u8, z: u8) -> Option<Vec<u8>> {
        self.chunk_capped(x, z, MAX_CHUNK_BYTES)
    }

    fn chunk_capped(&self, x: u8, z: u8, cap: usize) -> Option<Vec<u8>> {
        let (kind, data) = self.payload(x, z)?;
        match kind {
            1 => {
                let mut out = Vec::new();
                inflate_into(GzDecoder::new(data), cap, &mut out).then_some(out)
            }
            2 => {
                let mut out = Vec::new();
                inflate_zlib_into(data, cap, &mut out).then_some(out)
            }
            3 => (data.len() <= cap).then(|| data.to_vec()),
            _ => None,
        }
    }
}

/// Output is produced through a scratch buffer of this size.
const STEP: usize = 16 * 1024;

/// Appends one step of inflated bytes to `out`, keeping both its length within
/// `cap` and its capacity within `cap + STEP`. False when `cap` would be passed.
fn append_within(out: &mut Vec<u8>, step: &[u8], cap: usize) -> bool {
    if out.len() + step.len() > cap {
        return false;
    }
    if out.capacity() - out.len() < step.len() {
        let want = (out.len() + step.len())
            .max(out.capacity() * 2)
            .min(cap + STEP);
        out.reserve_exact(want - out.len());
    }
    out.extend_from_slice(step);
    true
}

/// Inflates a zlib stream that must end inside `data`. The reading decoders
/// report a cut stream as a clean end, so the end of the stream is checked here.
fn inflate_zlib_into(data: &[u8], cap: usize, out: &mut Vec<u8>) -> bool {
    let mut decoder = Decompress::new(true);
    let mut scratch = [0u8; STEP];
    loop {
        let (read_before, written_before) = (decoder.total_in(), decoder.total_out());
        let Ok(status) = decoder.decompress(
            &data[read_before as usize..],
            &mut scratch,
            FlushDecompress::None,
        ) else {
            return false;
        };
        let written = (decoder.total_out() - written_before) as usize;
        if !append_within(out, &scratch[..written], cap) {
            return false;
        }
        match status {
            Status::StreamEnd => return true,
            _ if decoder.total_in() == read_before && written == 0 => return false,
            _ => {}
        }
    }
}

/// Reads through a scratch buffer until the stream ends; false when it errors
/// or holds more than `cap` bytes.
fn inflate_into(mut reader: impl Read, cap: usize, out: &mut Vec<u8>) -> bool {
    let mut scratch = [0u8; STEP];
    loop {
        match reader.read(&mut scratch) {
            Ok(0) => return true,
            Ok(n) => {
                if !append_within(out, &scratch[..n], cap) {
                    return false;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use flate2::write::{GzEncoder, ZlibEncoder};
    use flate2::Compression;
    use std::io::Write;

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    /// A region file holding the given chunks (x, z, kind byte, stored data), each
    /// in the sectors it needs, laid out one after the other from sector 2.
    fn region_bytes(chunks: &[(u8, u8, u8, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = vec![0u8; HEADER];
        for (x, z, kind, data) in chunks {
            let sector = bytes.len() / SECTOR;
            let length = data.len() as u32 + 1;
            bytes.extend_from_slice(&length.to_be_bytes());
            bytes.push(*kind);
            bytes.extend_from_slice(data);
            let sectors = (bytes.len() - sector * SECTOR).div_ceil(SECTOR);
            bytes.resize((sector + sectors) * SECTOR, 0);
            let at = (*x as usize + *z as usize * 32) * 4;
            bytes[at..at + 3].copy_from_slice(&(sector as u32).to_be_bytes()[1..]);
            bytes[at + 3] = sectors as u8;
        }
        bytes
    }

    fn region_of(bytes: Vec<u8>) -> Region {
        Region { bytes }
    }

    /// Overwrites the location entry of a slot.
    fn set_entry(bytes: &mut [u8], x: usize, z: usize, offset: u32, count: u8) {
        let at = (x + z * 32) * 4;
        bytes[at..at + 3].copy_from_slice(&offset.to_be_bytes()[1..]);
        bytes[at + 3] = count;
    }

    const NBT: &[u8] = &[10, 0, 0, 1, 2, 3, 0];

    #[test]
    fn a_chunk_is_found_and_inflated() {
        let region = region_of(region_bytes(&[
            (0, 0, 2, zlib(NBT)),
            (31, 0, 1, gzip(NBT)),
            (5, 7, 3, NBT.to_vec()),
        ]));
        for (x, z) in [(0, 0), (31, 0), (5, 7)] {
            assert!(region.has_chunk(x, z));
            assert_eq!(region.chunk(x, z).as_deref(), Some(NBT));
        }
    }

    #[test]
    fn an_empty_slot_has_no_chunk() {
        let region = region_of(region_bytes(&[(0, 0, 2, zlib(NBT))]));
        assert!(!region.has_chunk(1, 0));
        assert_eq!(region.chunk(1, 0), None);
        assert!(!region.has_chunk(32, 0));
        assert_eq!(region.chunk(0, 32), None);
    }

    #[test]
    fn a_region_that_lies_gives_no_chunk_and_no_panic() {
        let good = || region_bytes(&[(0, 0, 2, zlib(NBT))]);
        let check = |bytes: Vec<u8>| {
            let region = region_of(bytes);
            assert_eq!(region.chunk(0, 0), None);
        };

        let mut b = good();
        let end = (b.len() / SECTOR) as u32;
        set_entry(&mut b, 0, 0, end + 10, 1);
        check(b); // offset past the file

        let mut b = good();
        set_entry(&mut b, 0, 0, u32::MAX >> 8, 255);
        check(b); // offset far past the file

        let mut b = good();
        set_entry(&mut b, 0, 0, 1, 1);
        check(b); // offset inside the header

        let mut b = good();
        set_entry(&mut b, 0, 0, 2, 0);
        check(b); // sector count 0

        let mut b = good();
        b[HEADER..HEADER + 4].copy_from_slice(&0u32.to_be_bytes());
        check(b); // length 0

        let mut b = good();
        b[HEADER..HEADER + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        check(b); // length past the file

        let mut b = good();
        b[HEADER..HEADER + 4].copy_from_slice(&(SECTOR as u32 + 1).to_be_bytes());
        b.resize(HEADER + 3 * SECTOR, 0);
        check(b); // length larger than its one sector, though inside the file

        check(region_bytes(&[(0, 0, 4, zlib(NBT))])); // LZ4
        check(region_bytes(&[(0, 0, 0x82, zlib(NBT))])); // external file
        check(region_bytes(&[(0, 0, 9, zlib(NBT))])); // unknown kind
        check(region_bytes(&[(0, 0, 0, zlib(NBT))])); // kind 0

        let noise: Vec<u8> = (0..5000u32)
            .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
            .collect();
        let whole = zlib(&noise);
        check(region_bytes(&[(
            0,
            0,
            2,
            whole[..whole.len() / 2].to_vec(),
        )])); // cut stream
        let whole = gzip(&noise);
        check(region_bytes(&[(
            0,
            0,
            1,
            whole[..whole.len() / 2].to_vec(),
        )])); // cut gzip
        let whole = gzip(NBT);
        check(region_bytes(&[(
            0,
            0,
            1,
            whole[..whole.len() - 4].to_vec(),
        )])); // gzip without its end
        let whole = zlib(NBT);
        check(region_bytes(&[(
            0,
            0,
            2,
            whole[..whole.len() - 2].to_vec(),
        )])); // zlib without its end
        check(region_bytes(&[(0, 0, 2, vec![1, 2, 3, 4, 5])])); // not a stream

        let mut b = good();
        b.truncate(HEADER + 3);
        check(b); // file ends inside the chunk header
    }

    #[test]
    fn a_stream_inflating_past_the_cap_gives_no_chunk() {
        let region = region_of(region_bytes(&[
            (0, 0, 2, zlib(&[0u8; 1000])),
            (1, 0, 1, gzip(&[0u8; 1000])),
            (2, 0, 3, vec![0u8; 1000]),
        ]));
        for x in 0..3 {
            assert_eq!(region.chunk_capped(x, 0, 999), None);
            assert_eq!(region.chunk_capped(x, 0, 1000).map(|c| c.len()), Some(1000));
        }
    }

    #[test]
    fn inflating_never_holds_much_more_than_the_cap() {
        let cap = 1000;
        let zeros = vec![0u8; 1024 * 1024];
        let mut out = Vec::new();
        assert!(!inflate_zlib_into(&zlib(&zeros), cap, &mut out));
        assert!(out.capacity() <= cap + 16 * 1024, "zlib {}", out.capacity());
        let mut out = Vec::new();
        assert!(!inflate_into(
            GzDecoder::new(&gzip(&zeros)[..]),
            cap,
            &mut out
        ));
        assert!(out.capacity() <= cap + 16 * 1024, "gzip {}", out.capacity());

        // a stream that fits is still returned whole, with a bounded capacity
        let cap = 100_000;
        let data: Vec<u8> = (0..cap as u32).map(|i| (i * 7 >> 3) as u8).collect();
        let mut out = Vec::new();
        assert!(inflate_zlib_into(&zlib(&data), cap, &mut out));
        assert_eq!(out, data);
        assert!(out.capacity() <= cap + 16 * 1024);
        let mut out = Vec::new();
        assert!(inflate_into(
            GzDecoder::new(&gzip(&data)[..]),
            cap,
            &mut out
        ));
        assert_eq!(out, data);
        assert!(out.capacity() <= cap + 16 * 1024);
    }

    #[test]
    fn a_file_that_is_no_region_is_not_read() {
        let dir = tempfile::tempdir().unwrap();

        let short = dir.path().join("short.mca");
        std::fs::write(&short, vec![0u8; 100]).unwrap();
        assert!(Region::read(&short).unwrap().is_none());

        assert!(Region::read(dir.path()).unwrap().is_none());
        assert!(Region::read(&dir.path().join("missing.mca"))
            .unwrap()
            .is_none());

        let real = dir.path().join("r.0.0.mca");
        std::fs::write(&real, region_bytes(&[(0, 0, 2, zlib(NBT))])).unwrap();
        let region = Region::read(&real).unwrap().unwrap();
        assert_eq!(region.chunk(0, 0).as_deref(), Some(NBT));

        #[cfg(unix)]
        {
            let fifo = dir.path().join("fifo.mca");
            let c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
            assert!(Region::read(&fifo).unwrap().is_none()); // returns, does not block

            let link = dir.path().join("link.mca");
            std::os::unix::fs::symlink(&real, &link).unwrap();
            assert!(Region::read(&link).unwrap().is_none());
        }
    }

    #[test]
    fn the_file_is_not_left_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.0.0.mca");
        std::fs::write(&path, region_bytes(&[(0, 0, 2, zlib(NBT))])).unwrap();

        let Ok(entries) = std::fs::read_dir("/proc/self/fd") else {
            return; // no /proc here
        };
        drop(entries);
        let open_to_path = || {
            let target = std::fs::canonicalize(&path).unwrap();
            std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(|e| std::fs::read_link(e.ok()?.path()).ok())
                .filter(|t| *t == target)
                .count()
        };
        assert_eq!(open_to_path(), 0);
        let region = Region::read(&path).unwrap();
        assert!(region.is_some());
        assert_eq!(open_to_path(), 0);
    }

    /// Runs only with `GDL_REAL_WORLDS` set to the folder of real worlds.
    #[test]
    fn real_region_files_inflate_to_compounds() {
        let Some(root) = std::env::var_os("GDL_REAL_WORLDS") else {
            return;
        };
        let mut files = Vec::new();
        let mut folders = vec![std::path::PathBuf::from(root)];
        while let Some(folder) = folders.pop() {
            for entry in std::fs::read_dir(&folder).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    folders.push(path);
                } else if path.extension().is_some_and(|e| e == "mca") {
                    files.push(path);
                }
            }
        }
        assert!(!files.is_empty(), "no .mca under GDL_REAL_WORLDS");
        let (mut regions, mut chunks, mut unreadable) = (0, 0, 0);
        for path in files {
            let Some(region) = Region::read(&path).unwrap() else {
                continue;
            };
            regions += 1;
            for z in 0..32 {
                for x in 0..32 {
                    if region.has_chunk(x, z) {
                        match region.chunk(x, z) {
                            Some(bytes) => {
                                assert_eq!(bytes.first(), Some(&10), "{path:?} chunk {x},{z}");
                                chunks += 1;
                            }
                            None => unreadable += 1,
                        }
                    }
                }
            }
        }
        println!(
            "real worlds: {regions} regions, {chunks} chunks inflated, {unreadable} listed but unreadable"
        );
        assert!(
            chunks * 100 >= (chunks + unreadable) * 95,
            "only {chunks} of {} listed chunks inflate",
            chunks + unreadable
        );
    }
}
