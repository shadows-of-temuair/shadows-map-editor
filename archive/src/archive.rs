use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;

use memmap2::Mmap;

const HEADER_SIZE: usize = 4;
const ENTRY_SIZE: usize = 17;
const NAME_LEN: usize = 13;

/// A single entry in a `.dat` file archive.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

/// Memory-mapped reader for `.dat` file archives.
///
/// The archive format is:
/// - `u32` entry count (includes a null terminator record)
/// - `count` entries of 17 bytes each: `u32 offset` + 13-byte null-terminated name
/// - File data packed sequentially; each file's size is derived from the next entry's offset
pub struct FileArchive {
    mmap: Mmap,
    entries: Vec<ArchiveEntry>,
    index: HashMap<String, usize>,
}

impl FileArchive {
    /// Opens and memory-maps a `.dat` archive, parsing its table of contents.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = File::open(path.as_ref())?;
        let mmap = unsafe { Mmap::map(&file)? };
        let data = &mmap[..];

        if data.len() < HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file too small for archive header",
            ));
        }

        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let toc_end = HEADER_SIZE + count * ENTRY_SIZE;

        if data.len() < toc_end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "file too small for TOC: need {} bytes, have {}",
                    toc_end,
                    data.len()
                ),
            ));
        }

        // The last entry is a null terminator (start = EOF, name = empty).
        // Real file count = count - 1.
        let file_count = count.saturating_sub(1);
        let mut entries = Vec::with_capacity(file_count);
        let mut index = HashMap::with_capacity(file_count);

        for i in 0..file_count {
            let base = HEADER_SIZE + i * ENTRY_SIZE;
            let offset =
                u32::from_le_bytes([data[base], data[base + 1], data[base + 2], data[base + 3]]);

            let name_bytes = &data[base + 4..base + 4 + NAME_LEN];
            let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
            let name = String::from_utf8_lossy(&name_bytes[..name_end]).into_owned();

            // Next entry's offset gives us this entry's end
            let next_base = HEADER_SIZE + (i + 1) * ENTRY_SIZE;
            let next_offset = u32::from_le_bytes([
                data[next_base],
                data[next_base + 1],
                data[next_base + 2],
                data[next_base + 3],
            ]);

            let size = next_offset.saturating_sub(offset);

            index.insert(name.clone(), entries.len());
            entries.push(ArchiveEntry { name, offset, size });
        }

        Ok(Self {
            mmap,
            entries,
            index,
        })
    }

    /// Returns the byte contents of a file by name, or `None` if not found.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        let &idx = self.index.get(name)?;
        let entry = &self.entries[idx];
        let start = entry.offset as usize;
        let end = start + entry.size as usize;

        if end <= self.mmap.len() {
            Some(&self.mmap[start..end])
        } else {
            None
        }
    }

    /// Returns all parsed entries.
    pub fn entries(&self) -> &[ArchiveEntry] {
        &self.entries
    }

    /// Number of files in the archive (excludes the null terminator).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the archive contains no files.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
