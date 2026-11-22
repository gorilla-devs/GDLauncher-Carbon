//! Reader and writer for uncompressed NBT (big-endian, as Minecraft stores it).
//!
//! Strings and compound keys stay raw bytes so that writing back what was read
//! reproduces the input exactly, even for text that is not valid modified UTF-8.

use std::fmt;

const MAX_DEPTH: usize = 512;

/// Each parsed tag costs far more memory than the byte it can come from, so the count is capped.
const MAX_TAGS: usize = 4_000_000;

#[derive(Debug, Clone)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(Vec<u8>),
    List { element: u8, items: Vec<Tag> },
    Compound(Vec<(Vec<u8>, Tag)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

// Floats compare by their bits so a NaN equals itself and a tree equals its own copy.
impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Tag::Byte(a), Tag::Byte(b)) => a == b,
            (Tag::Short(a), Tag::Short(b)) => a == b,
            (Tag::Int(a), Tag::Int(b)) => a == b,
            (Tag::Long(a), Tag::Long(b)) => a == b,
            (Tag::Float(a), Tag::Float(b)) => a.to_bits() == b.to_bits(),
            (Tag::Double(a), Tag::Double(b)) => a.to_bits() == b.to_bits(),
            (Tag::ByteArray(a), Tag::ByteArray(b)) => a == b,
            (Tag::String(a), Tag::String(b)) => a == b,
            (
                Tag::List {
                    element: ea,
                    items: ia,
                },
                Tag::List {
                    element: eb,
                    items: ib,
                },
            ) => ea == eb && ia == ib,
            (Tag::Compound(a), Tag::Compound(b)) => a == b,
            (Tag::IntArray(a), Tag::IntArray(b)) => a == b,
            (Tag::LongArray(a), Tag::LongArray(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Root {
    pub name: Vec<u8>,
    pub tag: Tag,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NbtError {
    UnexpectedEnd,
    UnknownTag(u8),
    BadLength,
    TooDeep,
    TooManyTags,
    TrailingBytes,
    RootNotCompound,
}

impl fmt::Display for NbtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NbtError::UnexpectedEnd => write!(f, "NBT data ends too early"),
            NbtError::UnknownTag(id) => write!(f, "NBT tag id {id} is unknown"),
            NbtError::BadLength => write!(f, "NBT length is negative or longer than the data"),
            NbtError::TooDeep => write!(f, "NBT is nested too deeply"),
            NbtError::TooManyTags => write!(f, "NBT holds too many tags"),
            NbtError::TrailingBytes => write!(f, "NBT has bytes after the root tag"),
            NbtError::RootNotCompound => write!(f, "NBT root is not a compound"),
        }
    }
}

impl std::error::Error for NbtError {}

impl Tag {
    pub fn get(&self, key: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(entries) => entries
                .iter()
                .find(|(name, _)| name == key.as_bytes())
                .map(|(_, tag)| tag),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Tag> {
        match self {
            Tag::Compound(entries) => entries
                .iter_mut()
                .find(|(name, _)| name == key.as_bytes())
                .map(|(_, tag)| tag),
            _ => None,
        }
    }

    fn id(&self) -> u8 {
        match self {
            Tag::Byte(_) => 1,
            Tag::Short(_) => 2,
            Tag::Int(_) => 3,
            Tag::Long(_) => 4,
            Tag::Float(_) => 5,
            Tag::Double(_) => 6,
            Tag::ByteArray(_) => 7,
            Tag::String(_) => 8,
            Tag::List { .. } => 9,
            Tag::Compound(_) => 10,
            Tag::IntArray(_) => 11,
            Tag::LongArray(_) => 12,
        }
    }
}

struct Cursor<'a> {
    rest: &'a [u8],
    tags: usize,
    max_tags: usize,
    max_depth: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8], max_tags: usize, max_depth: usize) -> Self {
        Cursor {
            rest: bytes,
            tags: 0,
            max_tags,
            max_depth,
        }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], NbtError> {
        if n > self.rest.len() {
            return Err(NbtError::UnexpectedEnd);
        }
        let (head, tail) = self.rest.split_at(n);
        self.rest = tail;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], NbtError> {
        Ok(self.take(N)?.try_into().expect("take returned N bytes"))
    }

    fn u8(&mut self) -> Result<u8, NbtError> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, NbtError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn i32(&mut self) -> Result<i32, NbtError> {
        Ok(i32::from_be_bytes(self.array()?))
    }

    /// A non-negative i32 length, for elements of at least `unit` bytes each,
    /// checked against what remains before anything is allocated.
    fn length(&mut self, unit: usize) -> Result<usize, NbtError> {
        let len = usize::try_from(self.i32()?).map_err(|_| NbtError::BadLength)?;
        let bytes = len.checked_mul(unit).ok_or(NbtError::BadLength)?;
        if bytes > self.rest.len() {
            return Err(NbtError::BadLength);
        }
        Ok(len)
    }

    fn name_bytes(&mut self) -> Result<&'a [u8], NbtError> {
        let len = self.u16()? as usize;
        self.take(len)
    }

    fn name(&mut self) -> Result<Vec<u8>, NbtError> {
        Ok(self.name_bytes()?.to_vec())
    }

    /// Counts one more tag against the limit, for built and skipped tags alike.
    fn count_tag(&mut self) -> Result<(), NbtError> {
        self.tags += 1;
        if self.tags > self.max_tags {
            return Err(NbtError::TooManyTags);
        }
        Ok(())
    }

    fn check_depth(&self, depth: usize) -> Result<(), NbtError> {
        if depth >= self.max_depth {
            return Err(NbtError::TooDeep);
        }
        Ok(())
    }

    /// The element id and length of a list, as checked for every list.
    fn list_header(&mut self) -> Result<(u8, usize), NbtError> {
        let element = self.u8()?;
        if element > 12 {
            return Err(NbtError::UnknownTag(element));
        }
        // Every element takes at least one byte, so the length is bounded by the input.
        let len = self.length(1)?;
        if element == 0 && len > 0 {
            return Err(NbtError::UnknownTag(0));
        }
        Ok((element, len))
    }

    /// Checks the depth of a list starting at `depth` and reads its header.
    fn start_list(&mut self, depth: usize) -> Result<(u8, usize), NbtError> {
        self.check_depth(depth)?;
        self.list_header()
    }

    /// The id of the next entry of a compound, `None` at its end.
    fn entry_id(&mut self) -> Result<Option<u8>, NbtError> {
        let id = self.u8()?;
        if id == 0 {
            return Ok(None);
        }
        if id > 12 {
            return Err(NbtError::UnknownTag(id));
        }
        Ok(Some(id))
    }
}

/// Walks one payload with every bounds check; builds it only when `build` is set
/// (and returns `None` otherwise, without allocating for arrays, lists or strings).
fn walk_payload(
    cur: &mut Cursor,
    id: u8,
    depth: usize,
    build: bool,
) -> Result<Option<Tag>, NbtError> {
    cur.count_tag()?;
    let tag = match id {
        1 => Tag::Byte(i8::from_be_bytes(cur.array()?)),
        2 => Tag::Short(i16::from_be_bytes(cur.array()?)),
        3 => Tag::Int(cur.i32()?),
        4 => Tag::Long(i64::from_be_bytes(cur.array()?)),
        5 => Tag::Float(f32::from_bits(u32::from_be_bytes(cur.array()?))),
        6 => Tag::Double(f64::from_bits(u64::from_be_bytes(cur.array()?))),
        7 => {
            let len = cur.length(1)?;
            let raw = cur.take(len)?;
            if !build {
                return Ok(None);
            }
            Tag::ByteArray(raw.to_vec())
        }
        8 => {
            let raw = cur.name_bytes()?;
            if !build {
                return Ok(None);
            }
            Tag::String(raw.to_vec())
        }
        9 => {
            let (element, len) = cur.start_list(depth)?;
            let mut items = Vec::with_capacity(if build { len.min(4096) } else { 0 });
            for _ in 0..len {
                if let Some(item) = walk_payload(cur, element, depth + 1, build)? {
                    items.push(item);
                }
            }
            Tag::List { element, items }
        }
        10 => {
            cur.check_depth(depth)?;
            let mut entries = Vec::new();
            while let Some(id) = cur.entry_id()? {
                let name = cur.name_bytes()?;
                if let Some(tag) = walk_payload(cur, id, depth + 1, build)? {
                    entries.push((name.to_vec(), tag));
                }
            }
            Tag::Compound(entries)
        }
        11 => {
            let len = cur.length(4)?;
            let raw = cur.take(len * 4)?;
            if !build {
                return Ok(None);
            }
            Tag::IntArray(
                raw.chunks_exact(4)
                    .map(|c| i32::from_be_bytes(c.try_into().unwrap()))
                    .collect(),
            )
        }
        12 => {
            let len = cur.length(8)?;
            let raw = cur.take(len * 8)?;
            if !build {
                return Ok(None);
            }
            Tag::LongArray(
                raw.chunks_exact(8)
                    .map(|c| i64::from_be_bytes(c.try_into().unwrap()))
                    .collect(),
            )
        }
        other => return Err(NbtError::UnknownTag(other)),
    };
    Ok(build.then_some(tag))
}

fn read_payload(cur: &mut Cursor, id: u8, depth: usize) -> Result<Tag, NbtError> {
    Ok(walk_payload(cur, id, depth, true)?.expect("built"))
}

/// Reads the root header: it must be a compound. Returns its name.
fn read_root_header<'a>(cur: &mut Cursor<'a>) -> Result<&'a [u8], NbtError> {
    let id = cur.u8()?;
    if id != 10 {
        return Err(if id > 12 || id == 0 {
            NbtError::UnknownTag(id)
        } else {
            NbtError::RootNotCompound
        });
    }
    cur.name_bytes()
}

/// Reads uncompressed NBT whose root is a compound.
pub fn read(bytes: &[u8]) -> Result<Root, NbtError> {
    let mut cur = Cursor::new(bytes, MAX_TAGS, MAX_DEPTH);
    let name = read_root_header(&mut cur)?.to_vec();
    let tag = read_payload(&mut cur, 10, 0)?;
    if !cur.rest.is_empty() {
        return Err(NbtError::TrailingBytes);
    }
    Ok(Root { name, tag })
}

/// What `read_keeping` does with a named tag inside a compound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// Build the tag whole.
    All,
    /// Build a compound, or a list of compounds, and ask about each entry inside it.
    /// Any other tag is built whole.
    Descend,
    /// Walk the tag with every check but build nothing; it is left out of the result.
    Skip,
}

/// Bounds on what one `read_keeping` call may walk, skipped tags included.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_tags: usize,
    pub max_depth: usize,
}

/// Builds the entries of a compound whose payload starts at the cursor, asking `keep`
/// about each with `path` plus the entry's key.
fn read_compound_keeping<'a>(
    cur: &mut Cursor<'a>,
    depth: usize,
    path: &mut Vec<&'a [u8]>,
    keep: &dyn Fn(&[&[u8]]) -> Keep,
) -> Result<Tag, NbtError> {
    cur.count_tag()?;
    cur.check_depth(depth)?;
    let mut entries = Vec::new();
    while let Some(id) = cur.entry_id()? {
        let name = cur.name_bytes()?;
        path.push(name);
        let verdict = keep(path);
        let built = match verdict {
            Keep::All => Some(read_payload(cur, id, depth + 1)?),
            Keep::Skip => walk_payload(cur, id, depth + 1, false)?,
            Keep::Descend => match id {
                10 => Some(read_compound_keeping(cur, depth + 1, path, keep)?),
                // A list is only taken apart when its members are compounds.
                9 if cur.rest.first() == Some(&10) => {
                    cur.count_tag()?;
                    let (element, len) = cur.start_list(depth + 1)?;
                    let mut items = Vec::with_capacity(len.min(4096));
                    for _ in 0..len {
                        items.push(read_compound_keeping(cur, depth + 2, path, keep)?);
                    }
                    Some(Tag::List { element, items })
                }
                _ => Some(read_payload(cur, id, depth + 1)?),
            },
        };
        path.pop();
        if let Some(tag) = built {
            entries.push((name.to_vec(), tag));
        }
    }
    Ok(Tag::Compound(entries))
}

/// Reads uncompressed NBT whose root is a compound, building only the parts `keep` asks for.
///
/// `keep` gets the compound keys from the root's content down to and including the tag in
/// question; the root's own name is not part of the path. The members of a list of
/// compounds are asked with the list's path plus their own key.
pub fn read_keeping(
    bytes: &[u8],
    keep: &dyn Fn(&[&[u8]]) -> Keep,
    limits: Limits,
) -> Result<Root, NbtError> {
    let mut cur = Cursor::new(bytes, limits.max_tags, limits.max_depth);
    let name = read_root_header(&mut cur)?.to_vec();
    let tag = read_compound_keeping(&mut cur, 0, &mut Vec::new(), keep)?;
    if !cur.rest.is_empty() {
        return Err(NbtError::TrailingBytes);
    }
    Ok(Root { name, tag })
}

fn write_payload(out: &mut Vec<u8>, tag: &Tag) {
    match tag {
        Tag::Byte(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Short(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Int(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Long(v) => out.extend_from_slice(&v.to_be_bytes()),
        Tag::Float(v) => out.extend_from_slice(&v.to_bits().to_be_bytes()),
        Tag::Double(v) => out.extend_from_slice(&v.to_bits().to_be_bytes()),
        Tag::ByteArray(v) => {
            out.extend_from_slice(&(v.len() as i32).to_be_bytes());
            out.extend_from_slice(v);
        }
        Tag::String(v) => {
            out.extend_from_slice(&(v.len() as u16).to_be_bytes());
            out.extend_from_slice(v);
        }
        Tag::List { element, items } => {
            out.push(*element);
            out.extend_from_slice(&(items.len() as i32).to_be_bytes());
            for item in items {
                write_payload(out, item);
            }
        }
        Tag::Compound(entries) => {
            for (name, value) in entries {
                out.push(value.id());
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
                write_payload(out, value);
            }
            out.push(0);
        }
        Tag::IntArray(v) => {
            out.extend_from_slice(&(v.len() as i32).to_be_bytes());
            for x in v {
                out.extend_from_slice(&x.to_be_bytes());
            }
        }
        Tag::LongArray(v) => {
            out.extend_from_slice(&(v.len() as i32).to_be_bytes());
            for x in v {
                out.extend_from_slice(&x.to_be_bytes());
            }
        }
    }
}

pub fn write(root: &Root) -> Vec<u8> {
    let mut out = vec![root.tag.id()];
    out.extend_from_slice(&(root.name.len() as u16).to_be_bytes());
    out.extend_from_slice(&root.name);
    write_payload(&mut out, &root.tag);
    out
}

/// Decodes Java's modified UTF-8: NUL is `C0 80` and characters beyond the
/// basic plane are two three-byte surrogates. Plain UTF-8 written by another
/// tool is accepted too. `None` when the bytes are neither.
pub fn decode_string(raw: &[u8]) -> Option<String> {
    decode_modified(raw).or_else(|| std::str::from_utf8(raw).ok().map(str::to_string))
}

fn decode_modified(raw: &[u8]) -> Option<String> {
    let mut units: Vec<u16> = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let b = raw[i];
        let cont = |j: usize| {
            raw.get(j)
                .filter(|c| *c & 0xC0 == 0x80)
                .map(|c| (*c & 0x3F) as u16)
        };
        if b & 0x80 == 0 {
            units.push(b as u16);
            i += 1;
        } else if b & 0xE0 == 0xC0 {
            units.push(((b & 0x1F) as u16) << 6 | cont(i + 1)?);
            i += 2;
        } else if b & 0xF0 == 0xE0 {
            units.push(((b & 0x0F) as u16) << 12 | cont(i + 1)? << 6 | cont(i + 2)?);
            i += 3;
        } else {
            return None;
        }
    }
    String::from_utf16(&units).ok()
}

pub fn encode_string(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for unit in text.encode_utf16() {
        match unit {
            1..=0x7F => out.push(unit as u8),
            0 | 0x80..=0x7FF => {
                out.push(0xC0 | (unit >> 6) as u8);
                out.push(0x80 | (unit & 0x3F) as u8);
            }
            _ => {
                out.push(0xE0 | (unit >> 12) as u8);
                out.push(0x80 | ((unit >> 6) & 0x3F) as u8);
                out.push(0x80 | (unit & 0x3F) as u8);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(out: &mut Vec<u8>, id: u8, name: &str) {
        out.push(id);
        out.extend_from_slice(&(name.len() as u16).to_be_bytes());
        out.extend_from_slice(name.as_bytes());
    }

    fn string_payload(out: &mut Vec<u8>, raw: &[u8]) {
        out.extend_from_slice(&(raw.len() as u16).to_be_bytes());
        out.extend_from_slice(raw);
    }

    fn every_tag_type_fixture() -> Vec<u8> {
        let mut b = vec![10, 0, 0];
        named(&mut b, 1, "byte");
        b.push(0xFE);
        named(&mut b, 2, "short");
        b.extend_from_slice(&(-1234i16).to_be_bytes());
        named(&mut b, 3, "int");
        b.extend_from_slice(&(-70000i32).to_be_bytes());
        named(&mut b, 4, "long");
        b.extend_from_slice(&i64::MIN.to_be_bytes());
        named(&mut b, 5, "float");
        b.extend_from_slice(&0x7FC0_0001u32.to_be_bytes()); // a NaN with a payload
        named(&mut b, 6, "double");
        b.extend_from_slice(&1.5f64.to_bits().to_be_bytes());
        named(&mut b, 7, "bytes");
        b.extend_from_slice(&3i32.to_be_bytes());
        b.extend_from_slice(&[1, 2, 3]);
        named(&mut b, 8, "string");
        let mut text = "é日本".as_bytes().to_vec();
        text.extend_from_slice(&[0xC0, 0x80]);
        // 😀 (U+1F600) as the surrogates D83D DE00, three bytes each
        text.extend_from_slice(&[0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80]);
        string_payload(&mut b, &text);
        named(&mut b, 9, "empty_list");
        b.push(0);
        b.extend_from_slice(&0i32.to_be_bytes());
        named(&mut b, 9, "compounds");
        b.push(10);
        b.extend_from_slice(&2i32.to_be_bytes());
        named(&mut b, 3, "a");
        b.extend_from_slice(&1i32.to_be_bytes());
        b.push(0);
        b.push(0);
        named(&mut b, 9, "nested_lists");
        b.push(9);
        b.extend_from_slice(&1i32.to_be_bytes());
        b.push(1);
        b.extend_from_slice(&2i32.to_be_bytes());
        b.extend_from_slice(&[7, 8]);
        named(&mut b, 11, "ints");
        b.extend_from_slice(&2i32.to_be_bytes());
        b.extend_from_slice(&1i32.to_be_bytes());
        b.extend_from_slice(&(-2i32).to_be_bytes());
        named(&mut b, 12, "longs");
        b.extend_from_slice(&2i32.to_be_bytes());
        b.extend_from_slice(&5i64.to_be_bytes());
        b.extend_from_slice(&(-6i64).to_be_bytes());
        b.push(0);
        b
    }

    #[test]
    fn every_tag_type_round_trips_byte_for_byte() {
        let b = every_tag_type_fixture();

        let root = read(&b).unwrap();
        assert_eq!(write(&root), b);
        assert!(matches!(root.tag.get("float"), Some(Tag::Float(f)) if f.is_nan()));
    }

    #[test]
    fn strings_use_javas_modified_utf8() {
        assert_eq!(
            decode_string(&encode_string("a\0😀é")),
            Some("a\0😀é".to_string())
        );
        assert_eq!(encode_string("\0"), [0xC0, 0x80]);
        assert_eq!(
            decode_string("😀 world".as_bytes()),
            Some("😀 world".to_string())
        );
        assert_eq!(decode_string(&[0xFF]), None);
        assert_eq!(decode_string(&[0xE0, 0x80]), None);
    }

    #[test]
    fn hostile_input_is_refused_not_trusted() {
        let cases: Vec<(&str, Vec<u8>)> = {
            let mut tag13 = vec![10, 0, 0];
            named(&mut tag13, 13, "x");
            tag13.push(0);

            let mut big_list = vec![10, 0, 0];
            named(&mut big_list, 9, "x");
            big_list.push(1);
            big_list.extend_from_slice(&i32::MAX.to_be_bytes());
            big_list.push(0);

            let mut long_bytes = vec![10, 0, 0];
            named(&mut long_bytes, 7, "x");
            long_bytes.extend_from_slice(&1000i32.to_be_bytes());
            long_bytes.extend_from_slice(&[1, 2, 3]);
            long_bytes.push(0);

            let mut negative = vec![10, 0, 0];
            named(&mut negative, 11, "x");
            negative.extend_from_slice(&(-1i32).to_be_bytes());
            negative.push(0);

            let mut deep = vec![10, 0, 0];
            for _ in 0..600 {
                named(&mut deep, 10, "n");
            }
            deep.extend(std::iter::repeat(0).take(601));

            let mut trailing = vec![10, 0, 0, 0];
            trailing.push(0);

            let mut int_root = vec![3, 0, 0];
            int_root.extend_from_slice(&1i32.to_be_bytes());

            vec![
                ("tag id 13", tag13),
                ("list of i32::MAX", big_list),
                ("byte array past the input", long_bytes),
                ("negative length", negative),
                ("600 nested compounds", deep),
                ("trailing byte", trailing),
                ("int root", int_root),
                ("empty input", vec![]),
            ]
        };
        for (what, bytes) in cases {
            assert!(read(&bytes).is_err(), "{what} must be refused");
        }
    }

    fn byte_list(len: usize) -> Vec<u8> {
        let mut b = vec![10, 0, 0];
        named(&mut b, 9, "x");
        b.push(1);
        b.extend_from_slice(&(len as i32).to_be_bytes());
        b.extend(std::iter::repeat(7).take(len));
        b.push(0);
        b
    }

    #[test]
    fn a_list_with_millions_of_tags_is_refused() {
        assert_eq!(read(&byte_list(5_000_000)), Err(NbtError::TooManyTags));
    }

    #[test]
    fn a_list_of_a_hundred_thousand_tags_round_trips() {
        let bytes = byte_list(100_000);
        assert_eq!(write(&read(&bytes).unwrap()), bytes);
    }

    fn keep_paths(
        wanted: &'static [&'static [&'static str]],
        descend: &'static [&'static [&'static str]],
    ) -> impl Fn(&[&[u8]]) -> Keep {
        move |path| {
            let is = |list: &[&[&str]]| {
                list.iter().any(|p| {
                    p.len() == path.len() && p.iter().zip(path).all(|(a, b)| a.as_bytes() == *b)
                })
            };
            if is(wanted) {
                Keep::All
            } else if is(descend) {
                Keep::Descend
            } else {
                Keep::Skip
            }
        }
    }

    fn int_entry(out: &mut Vec<u8>, name: &str, v: i32) {
        named(out, 3, name);
        out.extend_from_slice(&v.to_be_bytes());
    }

    fn partial_fixture() -> Vec<u8> {
        let mut b = vec![10, 0, 0];
        named(&mut b, 10, "Data");
        int_entry(&mut b, "Keep", 1);
        named(&mut b, 7, "Drop");
        b.extend_from_slice(&(5 * 1024 * 1024i32).to_be_bytes());
        b.extend(std::iter::repeat(9).take(5 * 1024 * 1024));
        named(&mut b, 10, "Inner");
        int_entry(&mut b, "A", 2);
        int_entry(&mut b, "B", 3);
        b.push(0);
        b.push(0);
        named(&mut b, 9, "Sections");
        b.push(10);
        b.extend_from_slice(&3i32.to_be_bytes());
        for y in 0..3 {
            int_entry(&mut b, "Y", y);
            named(&mut b, 7, "Blob");
            b.extend_from_slice(&4i32.to_be_bytes());
            b.extend_from_slice(&[1, 2, 3, 4]);
            b.push(0);
        }
        b.push(0);
        b
    }

    const LIMITS: Limits = Limits {
        max_tags: 1000,
        max_depth: 64,
    };

    #[test]
    fn only_the_named_parts_are_built() {
        let keep = keep_paths(
            &[
                &["Data", "Keep"],
                &["Data", "Inner", "A"],
                &["Sections", "Y"],
            ],
            &[&["Data"], &["Data", "Inner"], &["Sections"]],
        );
        let root = read_keeping(&partial_fixture(), &keep, LIMITS).unwrap();
        let expected = Tag::Compound(vec![
            (
                b"Data".to_vec(),
                Tag::Compound(vec![
                    (b"Keep".to_vec(), Tag::Int(1)),
                    (
                        b"Inner".to_vec(),
                        Tag::Compound(vec![(b"A".to_vec(), Tag::Int(2))]),
                    ),
                ]),
            ),
            (
                b"Sections".to_vec(),
                Tag::List {
                    element: 10,
                    items: (0..3)
                        .map(|y| Tag::Compound(vec![(b"Y".to_vec(), Tag::Int(y))]))
                        .collect(),
                },
            ),
        ]);
        assert_eq!(root.tag, expected);
    }

    #[test]
    fn a_skipped_part_is_still_checked() {
        let skip_all = |_: &[&[u8]]| Keep::Skip;

        let mut long_bytes = vec![10, 0, 0];
        named(&mut long_bytes, 7, "x");
        long_bytes.extend_from_slice(&1000i32.to_be_bytes());
        long_bytes.extend_from_slice(&[1, 2, 3]);
        long_bytes.push(0);
        assert!(read_keeping(&long_bytes, &skip_all, LIMITS).is_err());

        let mut bad_list = vec![10, 0, 0];
        named(&mut bad_list, 9, "x");
        bad_list.push(13);
        bad_list.extend_from_slice(&1i32.to_be_bytes());
        bad_list.push(0);
        assert!(read_keeping(&bad_list, &skip_all, LIMITS).is_err());

        let mut deep = vec![10, 0, 0];
        for _ in 0..100 {
            named(&mut deep, 10, "n");
        }
        deep.extend(std::iter::repeat(0).take(101));
        assert_eq!(
            read_keeping(&deep, &skip_all, LIMITS),
            Err(NbtError::TooDeep)
        );
    }

    #[test]
    fn the_tag_limit_counts_skipped_tags() {
        let mut b = vec![10, 0, 0];
        for i in 0..200 {
            int_entry(&mut b, &format!("k{i}"), i);
        }
        b.push(0);
        let limits = Limits {
            max_tags: 100,
            max_depth: 64,
        };
        assert_eq!(
            read_keeping(&b, &|_| Keep::Skip, limits),
            Err(NbtError::TooManyTags)
        );
    }

    #[test]
    fn everything_kept_equals_read() {
        let limits = Limits {
            max_tags: MAX_TAGS,
            max_depth: MAX_DEPTH,
        };
        for bytes in [partial_fixture(), every_tag_type_fixture()] {
            assert_eq!(read_keeping(&bytes, &|_| Keep::All, limits), read(&bytes));
        }
    }

    fn array_fixture(id: u8, unit: usize, len: usize, declared: usize) -> Vec<u8> {
        let mut b = vec![10, 0, 0];
        named(&mut b, id, "Big");
        b.extend_from_slice(&(declared as i32).to_be_bytes());
        b.extend(std::iter::repeat(0).take(len * unit));
        int_entry(&mut b, "After", 4);
        b.push(0);
        b
    }

    #[test]
    fn a_skipped_int_or_long_array_leaves_the_rest_intact() {
        let keep = |path: &[&[u8]]| {
            if path == [b"After".as_slice()] {
                Keep::All
            } else {
                Keep::Skip
            }
        };
        let expected = Tag::Compound(vec![(b"After".to_vec(), Tag::Int(4))]);
        for (id, unit) in [(11u8, 4usize), (12, 8)] {
            let n = 1024 * 1024 / unit;
            let root = read_keeping(&array_fixture(id, unit, n, n), &keep, LIMITS).unwrap();
            assert_eq!(root.tag, expected);
        }
    }

    #[test]
    fn a_skipped_int_or_long_array_past_the_input_is_refused() {
        for (id, unit) in [(11u8, 4usize), (12, 8)] {
            let bytes = array_fixture(id, unit, 2, 1000);
            assert!(read_keeping(&bytes, &|_| Keep::Skip, LIMITS).is_err());
        }
    }

    #[test]
    fn a_skipped_string_is_left_out() {
        let mut b = vec![10, 0, 0];
        named(&mut b, 8, "Name");
        string_payload(&mut b, b"hello");
        int_entry(&mut b, "After", 4);
        b.push(0);
        let keep = |path: &[&[u8]]| {
            if path == [b"After".as_slice()] {
                Keep::All
            } else {
                Keep::Skip
            }
        };
        let root = read_keeping(&b, &keep, LIMITS).unwrap();
        assert_eq!(
            root.tag,
            Tag::Compound(vec![(b"After".to_vec(), Tag::Int(4))])
        );
    }

    #[test]
    fn descending_into_a_list_that_is_not_of_compounds_builds_it_whole() {
        let mut b = vec![10, 0, 0];
        named(&mut b, 9, "Empty");
        b.push(0);
        b.extend_from_slice(&0i32.to_be_bytes());
        named(&mut b, 9, "Ints");
        b.push(3);
        b.extend_from_slice(&2i32.to_be_bytes());
        b.extend_from_slice(&7i32.to_be_bytes());
        b.extend_from_slice(&8i32.to_be_bytes());
        b.push(0);
        let root = read_keeping(&b, &|_| Keep::Descend, LIMITS).unwrap();
        assert_eq!(root, read(&b).unwrap());
    }
}
