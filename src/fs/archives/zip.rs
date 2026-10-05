// SPDX-FileCopyrightText: 2026 fxrdhan
// SPDX-License-Identifier: EUPL-1.2

//! The entries of a `.zip` archive, read from its central directory.
//!
//! A zip archive ends with a record pointing at its central directory, which
//! lists every entry's name and sizes; reading that is all a listing needs,
//! so nothing is decompressed and no dependency is taken on. ZIP64 archives,
//! whose counts, sizes or offsets outgrow 32 bits, keep the larger values in
//! records and extra fields of their own, which are read too.
//!
//! The layout is APPNOTE.TXT, the PKWARE specification, sections 4.3.12
//! (central directory header), 4.3.14 and 4.3.15 (ZIP64 end records),
//! 4.3.16 (end of central directory) and 4.5.3 (ZIP64 extra field).

use std::io::{self, BufReader, Read, Seek, SeekFrom};

use super::{ArchiveEntry, ArchiveListing, MAX_ENTRIES};

const END_SIGNATURE: u32 = 0x0605_4b50;
const END_LEN: usize = 22;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const ZIP64_LOCATOR_LEN: usize = 20;
const ZIP64_END_SIGNATURE: u32 = 0x0606_4b50;
const ZIP64_END_LEN: usize = 56;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const CENTRAL_LEN: usize = 46;
const ZIP64_EXTRA_ID: u16 = 0x0001;

/// The general purpose flag saying a name is UTF-8 rather than CP437.
const UTF8_NAME: u16 = 1 << 11;

fn invalid(what: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}

fn le16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn le32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn le64(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// Where the central directory is, and how many entries it says it holds.
struct Directory {
    entries: u64,
    size: u64,
    offset: u64,
    /// Where the record that follows the directory starts.
    end: u64,
}

/// Reads the entries of the zip archive `file`.
pub fn read_entries<R: Read + Seek>(mut file: R) -> io::Result<ArchiveListing> {
    let directory = find_directory(&mut file)?;
    let offset = locate_directory(&mut file, &directory)?;

    file.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::new(file.take(directory.size));
    let mut entries = Vec::new();
    let mut truncated = false;

    for _ in 0..directory.entries {
        match read_central_header(&mut reader)? {
            Header::File(entry) => {
                if entries.len() >= MAX_ENTRIES {
                    truncated = true;
                    break;
                }
                entries.push(entry);
            }
            Header::Directory => {}
            // A record cut short or out of place ends the listing, keeping
            // what was read before it, as a corrupt tail of a tar archive
            // does.
            Header::Broken => break,
        }
    }

    Ok(ArchiveListing { entries, truncated })
}

/// Finds the central directory from the end record, which is the last thing
/// in the archive but its comment, and from the ZIP64 end record when the
/// end record's fields are saturated.
fn find_directory<R: Read + Seek>(file: &mut R) -> io::Result<Directory> {
    let len = file.seek(SeekFrom::End(0))?;
    // The comment holds at most 65535 bytes.
    let tail_len = len.min((END_LEN + 0xFFFF) as u64);
    if tail_len < END_LEN as u64 {
        return Err(invalid("too short for a zip archive"));
    }
    let tail_start = len - tail_len;
    file.seek(SeekFrom::Start(tail_start))?;
    let mut tail = vec![0; tail_len as usize];
    file.read_exact(&mut tail)?;

    let at = (0..=tail.len() - END_LEN)
        .rev()
        .find(|&at| le32(&tail, at) == Some(END_SIGNATURE))
        .ok_or_else(|| invalid("no end of central directory record"))?;
    let end = &tail[at..];
    let end_position = tail_start + at as u64;
    let short = || invalid("short end record");
    let mut directory = Directory {
        entries: le16(end, 10).map(u64::from).ok_or_else(short)?,
        size: le32(end, 12).map(u64::from).ok_or_else(short)?,
        offset: le32(end, 16).map(u64::from).ok_or_else(short)?,
        end: end_position,
    };

    let saturated = directory.entries == 0xFFFF
        || directory.size == 0xFFFF_FFFF
        || directory.offset == 0xFFFF_FFFF;
    if saturated && let Some(zip64) = find_zip64_directory(file, end_position)? {
        directory = zip64;
    }

    if directory
        .offset
        .checked_add(directory.size)
        .is_none_or(|directory_end| directory_end > len)
    {
        return Err(invalid("central directory past the end of the archive"));
    }
    Ok(directory)
}

/// The ZIP64 end record, found through the locator that sits just before
/// the end record, if the archive has one.
fn find_zip64_directory<R: Read + Seek>(
    file: &mut R,
    end_position: u64,
) -> io::Result<Option<Directory>> {
    let Some(locator_position) = end_position.checked_sub(ZIP64_LOCATOR_LEN as u64) else {
        return Ok(None);
    };
    file.seek(SeekFrom::Start(locator_position))?;
    let mut locator = [0; ZIP64_LOCATOR_LEN];
    file.read_exact(&mut locator)?;
    if le32(&locator, 0) != Some(ZIP64_LOCATOR_SIGNATURE) {
        return Ok(None);
    }

    // The record is where the locator says, or, with something put in
    // front of the archive, just before the locator, where it is written.
    let declared = le64(&locator, 8).ok_or_else(|| invalid("short ZIP64 locator"))?;
    let mut record = [0; ZIP64_END_LEN];
    let mut record_position = None;
    for position in [
        Some(declared),
        locator_position.checked_sub(ZIP64_END_LEN as u64),
    ]
    .into_iter()
    .flatten()
    {
        file.seek(SeekFrom::Start(position))?;
        if read_all(file, &mut record)?.is_some() && le32(&record, 0) == Some(ZIP64_END_SIGNATURE) {
            record_position = Some(position);
            break;
        }
    }
    let record_position =
        record_position.ok_or_else(|| invalid("ZIP64 locator points at no ZIP64 end record"))?;
    let short = || invalid("short ZIP64 end record");
    Ok(Some(Directory {
        entries: le64(&record, 32).ok_or_else(short)?,
        size: le64(&record, 40).ok_or_else(short)?,
        offset: le64(&record, 48).ok_or_else(short)?,
        end: record_position,
    }))
}

/// Where the central directory starts. That is where the end record says,
/// unless something was put in front of the archive, as a self-extracting
/// one has: then every offset is short by its length, and the directory is
/// found where it ends, just before the end record, as `unzip` finds it.
fn locate_directory<R: Read + Seek>(file: &mut R, directory: &Directory) -> io::Result<u64> {
    if directory.entries == 0 || starts_a_header(file, directory.offset)? {
        return Ok(directory.offset);
    }
    match directory.end.checked_sub(directory.size) {
        Some(moved) if moved != directory.offset && starts_a_header(file, moved)? => Ok(moved),
        _ => Ok(directory.offset),
    }
}

/// Whether a central directory header starts at `position`.
fn starts_a_header<R: Read + Seek>(file: &mut R, position: u64) -> io::Result<bool> {
    file.seek(SeekFrom::Start(position))?;
    let mut signature = [0; 4];
    Ok(read_all(file, &mut signature)?.is_some() && le32(&signature, 0) == Some(CENTRAL_SIGNATURE))
}

/// What one central directory header turned out to be.
enum Header {
    File(ArchiveEntry),
    /// A directory, which is not listed.
    Directory,
    /// No whole header was left to read.
    Broken,
}

fn read_central_header<R: Read>(reader: &mut R) -> io::Result<Header> {
    let mut fixed = [0; CENTRAL_LEN];
    if read_all(reader, &mut fixed)?.is_none() || le32(&fixed, 0) != Some(CENTRAL_SIGNATURE) {
        return Ok(Header::Broken);
    }
    let short = || invalid("short central directory header");
    let flags = le16(&fixed, 8).ok_or_else(short)?;
    let size = le32(&fixed, 24).ok_or_else(short)?;
    let name_len = usize::from(le16(&fixed, 28).ok_or_else(short)?);
    let extra_len = usize::from(le16(&fixed, 30).ok_or_else(short)?);
    let comment_len = usize::from(le16(&fixed, 32).ok_or_else(short)?);

    let mut name = vec![0; name_len];
    let mut extra = vec![0; extra_len];
    let mut comment = vec![0; comment_len];
    if read_all(reader, &mut name)?.is_none()
        || read_all(reader, &mut extra)?.is_none()
        || read_all(reader, &mut comment)?.is_none()
    {
        return Ok(Header::Broken);
    }

    if name.last() == Some(&b'/') {
        return Ok(Header::Directory);
    }

    // A size too large for 32 bits is saturated here, and the real one is
    // the first value of the ZIP64 extra field.
    let size = if size == u32::MAX {
        zip64_size(&extra).unwrap_or(u64::from(size))
    } else {
        u64::from(size)
    };

    Ok(Header::File(ArchiveEntry {
        path: decode_name(&name, flags),
        size,
    }))
}

/// Fills `buffer`, or says it could not, without treating the end of the
/// data as an error.
fn read_all<R: Read>(reader: &mut R, buffer: &mut [u8]) -> io::Result<Option<()>> {
    match reader.read_exact(buffer) {
        Ok(()) => Ok(Some(())),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(e),
    }
}

/// The uncompressed size from a ZIP64 extra field, which comes first in it.
fn zip64_size(mut extra: &[u8]) -> Option<u64> {
    while extra.len() >= 4 {
        let id = le16(extra, 0)?;
        let len = usize::from(le16(extra, 2)?);
        let data = extra.get(4..4 + len)?;
        if id == ZIP64_EXTRA_ID {
            return le64(data, 0);
        }
        extra = &extra[4 + len..];
    }
    None
}

/// A name in UTF-8 when its flag says so, or when it is valid UTF-8 anyway,
/// as many writers set no flag; otherwise in CP437, as the specification
/// has it.
fn decode_name(name: &[u8], flags: u16) -> String {
    if flags & UTF8_NAME != 0 {
        return String::from_utf8_lossy(name).into_owned();
    }
    match std::str::from_utf8(name) {
        Ok(name) => name.to_owned(),
        Err(_) => name
            .iter()
            .map(|&byte| match byte {
                0..=0x7F => char::from(byte),
                high => CP437_HIGH[usize::from(high - 0x80)],
            })
            .collect(),
    }
}

/// Code page 437, from 0x80 up.
#[rustfmt::skip]
const CP437_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    /// One entry of an archive the tests build.
    struct Entry<'a> {
        name: &'a [u8],
        size: u64,
        flags: u16,
    }

    fn entry(name: &str, size: u64) -> Entry<'_> {
        Entry {
            name: name.as_bytes(),
            size,
            flags: 0,
        }
    }

    /// A zip archive of stored entries, filled with zeroes: enough for a
    /// reader that only looks at the central directory. With `zip64`, every
    /// size goes in a ZIP64 extra field and the end record is saturated.
    fn build(entries: &[Entry<'_>], zip64: bool, comment: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for e in entries {
            let offset = out.len() as u32;
            let size32 = if zip64 { u32::MAX } else { e.size as u32 };
            out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
            out.extend_from_slice(&[20, 0]);
            out.extend_from_slice(&e.flags.to_le_bytes());
            out.extend_from_slice(&[0; 10]);
            out.extend_from_slice(&size32.to_le_bytes());
            out.extend_from_slice(&size32.to_le_bytes());
            out.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(e.name);
            // Only the central directory is read, so the data can be short.
            out.extend(std::iter::repeat_n(0, e.size.min(16) as usize));

            let extra: Vec<u8> = if zip64 {
                [
                    &ZIP64_EXTRA_ID.to_le_bytes()[..],
                    &16_u16.to_le_bytes(),
                    &e.size.to_le_bytes(),
                    &e.size.to_le_bytes(),
                ]
                .concat()
            } else {
                Vec::new()
            };
            central.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
            central.extend_from_slice(&[20, 0, 20, 0]);
            central.extend_from_slice(&e.flags.to_le_bytes());
            central.extend_from_slice(&[0; 10]);
            central.extend_from_slice(&size32.to_le_bytes());
            central.extend_from_slice(&size32.to_le_bytes());
            central.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
            central.extend_from_slice(&(extra.len() as u16).to_le_bytes());
            // Comment length, disk, internal and external attributes.
            central.extend_from_slice(&[0; 10]);
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(e.name);
            central.extend_from_slice(&extra);
        }
        let directory_offset = out.len() as u64;
        let directory_size = central.len() as u64;
        out.extend_from_slice(&central);

        if zip64 {
            let record_position = out.len() as u64;
            out.extend_from_slice(&ZIP64_END_SIGNATURE.to_le_bytes());
            out.extend_from_slice(&44_u64.to_le_bytes());
            out.extend_from_slice(&[45, 0, 45, 0]);
            out.extend_from_slice(&[0; 8]);
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
            out.extend_from_slice(&directory_size.to_le_bytes());
            out.extend_from_slice(&directory_offset.to_le_bytes());
            out.extend_from_slice(&ZIP64_LOCATOR_SIGNATURE.to_le_bytes());
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&record_position.to_le_bytes());
            out.extend_from_slice(&1_u32.to_le_bytes());
        }

        let (count, size, offset) = if zip64 {
            (0xFFFF, u32::MAX, u32::MAX)
        } else {
            (
                entries.len() as u16,
                directory_size as u32,
                directory_offset as u32,
            )
        };
        out.extend_from_slice(&END_SIGNATURE.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        out.extend_from_slice(comment);
        out
    }

    fn listed(archive: Vec<u8>) -> io::Result<Vec<(String, u64)>> {
        read_entries(Cursor::new(archive)).map(|listing| {
            listing
                .entries
                .into_iter()
                .map(|e| (e.path, e.size))
                .collect()
        })
    }

    fn pairs(list: &[(&str, u64)]) -> Vec<(String, u64)> {
        list.iter().map(|&(n, s)| (n.to_owned(), s)).collect()
    }

    #[test]
    fn files_are_listed_in_order_and_directories_are_not() {
        let archive = build(
            &[
                entry("b.txt", 5),
                entry("nested/", 0),
                entry("nested/a.rs", 12),
            ],
            false,
            b"",
        );
        assert_eq!(
            listed(archive).unwrap(),
            pairs(&[("b.txt", 5), ("nested/a.rs", 12)])
        );
    }

    #[test]
    fn a_comment_after_the_end_record_is_skipped() {
        let archive = build(&[entry("a", 1)], false, &[b'x'; 300]);
        assert_eq!(listed(archive).unwrap(), pairs(&[("a", 1)]));
    }

    #[test]
    fn zip64_sizes_and_records_are_read() {
        let big = 5 * 1024 * 1024 * 1024;
        let archive = build(&[entry("huge.bin", big), entry("small", 3)], true, b"");
        assert_eq!(
            listed(archive).unwrap(),
            pairs(&[("huge.bin", big), ("small", 3)])
        );
    }

    #[test]
    fn names_are_utf8_or_else_cp437() {
        let flagged = Entry {
            name: "café.txt".as_bytes(),
            size: 1,
            flags: UTF8_NAME,
        };
        let unflagged_utf8 = entry("naïve.txt", 1);
        let cp437 = Entry {
            name: b"\x80\xa4\xe1.txt",
            size: 1,
            flags: 0,
        };
        let archive = build(&[flagged, unflagged_utf8, cp437], false, b"");
        assert_eq!(
            listed(archive).unwrap(),
            pairs(&[("café.txt", 1), ("naïve.txt", 1), ("Çñß.txt", 1)])
        );
    }

    #[test]
    fn a_long_archive_is_cut_short() {
        let names: Vec<String> = (0..=MAX_ENTRIES).map(|i| format!("f{i}")).collect();
        let entries: Vec<_> = names.iter().map(|n| entry(n, 0)).collect();
        let listing = read_entries(Cursor::new(build(&entries, false, b""))).unwrap();
        assert_eq!(listing.entries.len(), MAX_ENTRIES);
        assert!(listing.truncated);

        let listing =
            read_entries(Cursor::new(build(&entries[..MAX_ENTRIES], false, b""))).unwrap();
        assert_eq!(listing.entries.len(), MAX_ENTRIES);
        assert!(!listing.truncated);
    }

    #[test]
    fn data_put_in_front_of_the_archive_is_allowed_for() {
        for zip64 in [false, true] {
            let mut archive = vec![0x5a; 4096];
            archive.extend(build(&[entry("a.txt", 1), entry("b.txt", 2)], zip64, b""));
            assert_eq!(
                listed(archive).unwrap(),
                pairs(&[("a.txt", 1), ("b.txt", 2)]),
                "zip64: {zip64}"
            );
        }
    }

    #[test]
    fn what_is_not_a_zip_archive_is_an_error() {
        assert!(listed(Vec::new()).is_err());
        assert!(listed(b"not a zip archive at all, just some words".to_vec()).is_err());

        // An end record pointing past the end of the file.
        let mut archive = build(&[entry("a", 1)], false, b"");
        let at = archive.len() - END_LEN;
        archive[at + 16..at + 20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(listed(archive).is_err());
    }

    #[test]
    fn a_central_directory_cut_short_keeps_what_was_read() {
        let mut archive = build(&[entry("first", 1), entry("second", 2)], false, b"");
        // Corrupt the second header's signature.
        let second = archive
            .windows(4)
            .enumerate()
            .filter(|(_, w)| *w == CENTRAL_SIGNATURE.to_le_bytes())
            .nth(1)
            .unwrap()
            .0;
        archive[second] = 0;
        assert_eq!(listed(archive).unwrap(), pairs(&[("first", 1)]));
    }

    #[test]
    fn the_cp437_table_has_a_character_for_every_high_byte() {
        assert_eq!(CP437_HIGH.len(), 128);
        assert_eq!(CP437_HIGH[0], 'Ç');
        assert_eq!(CP437_HIGH[127], '\u{a0}');
    }
}
