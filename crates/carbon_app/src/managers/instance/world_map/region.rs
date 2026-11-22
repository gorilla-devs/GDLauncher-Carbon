//! Reads a Minecraft region file (`r.<x>.<z>.mca`): the whole file, or only its table and
//! one row of chunks, is read into memory and closed at once, then the inflated bytes of one
//! chunk are handed out at a time. Nothing in the file is trusted: every offset and length is
//! checked against the file's real size, every buffer sized from the file is reserved so that
//! a refusal is an unreadable region or chunk, and inflating is capped.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::{Decompress, FlushDecompress, Status};

const SECTOR: usize = 4096;
const HEADER: usize = 2 * SECTOR;

/// A region file above this size is left empty.
pub(super) const MAX_REGION_BYTES: u64 = 256 * 1024 * 1024;
/// A chunk inflating past this size is left empty.
pub(super) const MAX_CHUNK_BYTES: usize = 32 * 1024 * 1024;

/// What reading one row of a region holds at most: the table, then for each of its 32 chunks
/// the most sectors an entry can list and the 4 bytes a length may run past them.
pub(super) const MAX_ROW_BYTES: u64 = (HEADER + 32 * (255 * SECTOR + 4)) as u64;

pub(super) struct Region {
    bytes: Held,
}

enum Held {
    /// The whole file.
    Whole(Vec<u8>),
    /// The table, and the bytes each chunk of one row lists, by where they start in the file.
    Row {
        table: Vec<u8>,
        spans: Vec<(usize, Vec<u8>)>,
    },
}

impl Held {
    /// The bytes at `range` of the file, when they were read.
    fn get(&self, range: Range<usize>) -> Option<&[u8]> {
        match self {
            Held::Whole(bytes) => bytes.get(range),
            Held::Row { table, spans } => {
                if range.end <= table.len() {
                    return table.get(range);
                }
                spans.iter().find_map(|(start, bytes)| {
                    let from = range.start.checked_sub(*start)?;
                    bytes.get(from..from + range.len())
                })
            }
        }
    }
}

/// An empty buffer that can take `len` bytes; `None` when that much cannot be had.
fn reserved(len: usize) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(len).ok()?;
    Some(bytes)
}

fn refused() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::OutOfMemory)
}

/// Opens the region file at `path`; `None` for a missing file, a link, anything but a regular
/// file, a file shorter than the two header sectors or one above `MAX_REGION_BYTES`.
fn open(path: &Path) -> std::io::Result<Option<(File, u64)>> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if !meta.is_file() || meta.len() < HEADER as u64 || meta.len() > MAX_REGION_BYTES {
        return Ok(None);
    }
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
    Ok(Some((file, opened.len())))
}

/// Reads at most `len` bytes from where `file` is, fewer where it ends.
fn read_up_to(file: &mut File, len: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = reserved(len).ok_or_else(refused)?;
    file.take(len as u64).read_to_end(&mut bytes)?;
    Ok(bytes)
}

impl Region {
    /// The whole file; `None` as `open` says.
    pub(super) fn read(path: &Path) -> std::io::Result<Option<Region>> {
        let Some((file, len)) = open(path)? else {
            return Ok(None);
        };
        // The file may grow while it is read (the game is running): the cap holds
        // for what is actually read, not only for what the metadata said.
        let mut bytes = reserved(len as usize).ok_or_else(refused)?;
        file.take(MAX_REGION_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() < HEADER || bytes.len() as u64 > MAX_REGION_BYTES {
            return Ok(None);
        }
        Ok(Some(Region {
            bytes: Held::Whole(bytes),
        }))
    }

    /// The table and the chunks of row `z` (0 to 31) alone, which give those chunks as the
    /// whole file would: a region painted needs only the last row of the one to its north.
    /// `None` as `open` says.
    pub(super) fn read_row(path: &Path, z: u8) -> std::io::Result<Option<Region>> {
        let Some((mut file, len)) = open(path)? else {
            return Ok(None);
        };
        let table = read_up_to(&mut file, HEADER)?;
        if table.len() < HEADER || z >= 32 {
            return Ok(None);
        }
        // Chunks never share sectors, so a row's spans are no more than the file, but for
        // the 4 bytes each reads past its sectors: a row that claims more is a lie, and is
        // not held many times over.
        let most = len.saturating_add(32 * 4);
        let mut held = 0u64;
        let mut spans = Vec::new();
        for x in 0..32 {
            let at = (x + usize::from(z) * 32) * 4;
            let e = &table[at..at + 4];
            let offset = u32::from_be_bytes([0, e[0], e[1], e[2]]) as usize;
            let count = e[3] as usize;
            if offset < 2 || count == 0 {
                continue;
            }
            // `payload` reads the 4 bytes of the length and at most `count` sectors after them.
            let start = (offset * SECTOR) as u64;
            if start >= MAX_REGION_BYTES {
                continue;
            }
            file.seek(SeekFrom::Start(start))?;
            let bytes = read_up_to(&mut file, count * SECTOR + 4)?;
            held += bytes.len() as u64;
            if held > most {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "the row's chunks share sectors",
                ));
            }
            if !bytes.is_empty() {
                spans.push((start as usize, bytes));
            }
        }
        Ok(Some(Region {
            bytes: Held::Row { table, spans },
        }))
    }

    /// How many bytes of the file are held.
    #[cfg(test)]
    fn held(&self) -> usize {
        match &self.bytes {
            Held::Whole(bytes) => bytes.len(),
            Held::Row { table, spans } => {
                table.len() + spans.iter().map(|(_, bytes)| bytes.len()).sum::<usize>()
            }
        }
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
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn has_chunk(&self, x: u8, z: u8) -> bool {
        self.payload(x, z).is_some()
    }

    /// Whether the table has an entry for this slot at all, whatever its bounds: a listed
    /// slot that gives no `chunk` is damaged or not written yet, unlike an empty one.
    pub(super) fn lists_chunk(&self, x: u8, z: u8) -> bool {
        self.entry(x, z).is_some()
    }

    /// Whether the slot holds, within bounds, a chunk this reader does not read by design:
    /// LZ4 (kind 4), or one kept in an outside `.mcc` file (a known kind with its high bit
    /// set). Any other kind is damage.
    pub(super) fn left_out(&self, x: u8, z: u8) -> bool {
        self.payload(x, z)
            .is_some_and(|(kind, _)| matches!(kind, 4 | 0x81..=0x84))
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
            3 if data.len() <= cap => {
                let mut out = reserved(data.len())?;
                out.extend_from_slice(data);
                Some(out)
            }
            3 => None,
            _ => None,
        }
    }
}

/// Output is produced through a scratch buffer of this size.
const STEP: usize = 16 * 1024;

/// Appends one step of inflated bytes to `out`, keeping both its length within
/// `cap` and its capacity within `cap + STEP`. False when `cap` would be passed or the room
/// cannot be had.
fn append_within(out: &mut Vec<u8>, step: &[u8], cap: usize) -> bool {
    if out.len() + step.len() > cap {
        return false;
    }
    if out.capacity() - out.len() < step.len() {
        let want = (out.len() + step.len())
            .max(out.capacity() * 2)
            .min(cap + STEP);
        if out.try_reserve_exact(want - out.len()).is_err() {
            return false;
        }
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
pub(super) mod test {
    use super::*;
    use flate2::Compression;
    use flate2::write::{GzEncoder, ZlibEncoder};
    use std::io::Write;

    pub(in super::super) fn zlib(data: &[u8]) -> Vec<u8> {
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
    pub(in super::super) fn region_bytes(chunks: &[(u8, u8, u8, Vec<u8>)]) -> Vec<u8> {
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

    pub(in super::super) fn region_of(bytes: Vec<u8>) -> Region {
        Region {
            bytes: Held::Whole(bytes),
        }
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
    fn a_slot_is_listed_when_its_entry_is_not_zero() {
        let mut bytes = region_bytes(&[(0, 0, 2, zlib(NBT))]);
        set_entry(&mut bytes, 1, 0, 9999, 3);
        set_entry(&mut bytes, 2, 0, 0, 1);
        let region = region_of(bytes);
        assert!(region.lists_chunk(0, 0));
        // Listed but out of bounds, so `chunk` gives nothing and `has_chunk` says no.
        for x in [1, 2] {
            assert!(region.lists_chunk(x, 0));
            assert!(!region.has_chunk(x, 0));
        }
        assert!(!region.lists_chunk(3, 0));
        assert!(!region.lists_chunk(32, 0));
        assert!(!region.lists_chunk(0, 32));
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
        assert!(
            Region::read(&dir.path().join("missing.mca"))
                .unwrap()
                .is_none()
        );

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

    /// Bytes that do not compress, so a chunk of them takes as many sectors as it is long.
    fn noise(len: usize, seed: u32) -> Vec<u8> {
        (0..len as u32)
            .map(|i| (i.wrapping_add(seed).wrapping_mul(2654435761) >> 13) as u8)
            .collect()
    }

    #[test]
    fn the_last_row_alone_gives_the_chunks_the_whole_file_does() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.0.-1.mca");
        let mut bytes = region_bytes(&[
            // A large chunk outside the row, which reading the row must not hold.
            (0, 0, 3, noise(600_000, 1)),
            (0, 31, 2, zlib(NBT)),
            (7, 31, 1, gzip(NBT)),
            (31, 31, 3, noise(9000, 2)),
            (12, 31, 2, zlib(&noise(5000, 3))),
        ]);
        // Entries of the row that lie, as the whole file reads them.
        set_entry(&mut bytes, 3, 31, 9999, 3);
        set_entry(&mut bytes, 4, 31, 1, 1);
        set_entry(&mut bytes, 5, 31, 2, 0);
        let last = (bytes.len() / SECTOR - 1) as u32;
        set_entry(&mut bytes, 6, 31, last, 4); // runs past the end of the file
        std::fs::write(&path, &bytes).unwrap();

        let whole = Region::read(&path).unwrap().unwrap();
        let row = Region::read_row(&path, 31).unwrap().unwrap();
        for x in 0..32 {
            assert_eq!(row.chunk(x, 31), whole.chunk(x, 31), "slot {x},31");
        }
        assert!(row.chunk(0, 31).is_some() && row.chunk(31, 31).is_some());
        assert_eq!(row.chunk(0, 0), None, "only the row is read");
        assert!(whole.chunk(0, 0).is_some());
        assert!(
            row.held() < 64 * 1024,
            "{} bytes held of {}",
            row.held(),
            bytes.len()
        );

        assert!(
            Region::read_row(&dir.path().join("missing.mca"), 31)
                .unwrap()
                .is_none()
        );
        let short = dir.path().join("short.mca");
        std::fs::write(&short, vec![0u8; 100]).unwrap();
        assert!(Region::read_row(&short, 31).unwrap().is_none());
        #[cfg(unix)]
        {
            let link = dir.path().join("link.mca");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(Region::read_row(&link, 31).unwrap().is_none());
        }
    }

    #[test]
    fn a_row_whose_entries_share_sectors_is_refused_not_held_many_times() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.0.-1.mca");
        let mut bytes = region_bytes(&[(0, 31, 3, noise(1_000_000, 1))]);
        for x in 1..32 {
            set_entry(&mut bytes, x, 31, 2, 255);
        }
        std::fs::write(&path, &bytes).unwrap();
        let error = Region::read_row(&path, 31).err().expect("refused");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        // The whole file, read once, still gives what each entry names.
        assert!(Region::read(&path).unwrap().unwrap().chunk(0, 31).is_some());
    }

    #[test]
    fn only_a_known_kind_is_left_out_by_design() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.0.0.mca");
        let bytes = region_bytes(&[
            (0, 0, 4, noise(100, 1)),
            (1, 0, 0x82, vec![]),
            (2, 0, 0x80, noise(100, 2)),
            (3, 0, 0xff, noise(100, 3)),
            (4, 0, 9, noise(100, 4)),
            (5, 0, 2, zlib(NBT)),
        ]);
        std::fs::write(&path, &bytes).unwrap();
        let region = Region::read(&path).unwrap().unwrap();
        let left_out: Vec<bool> = (0..6).map(|x| region.left_out(x, 0)).collect();
        assert_eq!(left_out, [true, true, false, false, false, false]);
    }

    #[test]
    fn a_reservation_that_cannot_be_had_is_refused_not_an_abort() {
        assert!(reserved(usize::MAX).is_none());
        assert!(reserved(isize::MAX as usize).is_none());
        assert_eq!(reserved(10).map(|v| v.capacity() >= 10), Some(true));
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
}
