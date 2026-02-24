use std::fmt;

/// Signature indicating Huffman compression: `0xFF02AA55` (little-endian).
const HPF_SIGNATURE: u32 = 0xFF02AA55;

/// A decoded HPF (Huffman Picture File) sprite.
///
/// HPF files store 8-bit palette-indexed images compressed with an adaptive
/// Huffman coding scheme. The format originates from a mid-90s game engine.
///
/// File layout (compressed):
///   - Bytes 0–3: signature `0xFF02AA55` (LE) indicating Huffman compression
///   - Bytes 4+:  Huffman-compressed bitstream
///
/// After decompression (or if uncompressed):
///   - Bytes 0–7: 8-byte header (reserved; width is always 28)
///   - Bytes 8+:  pixel data (8-bit palette-indexed, row-major)
///
/// The pixel width is fixed at 28 (half a ground tile). The pixel height is
/// derived from the data length: `(decompressed_size - 8) / 28`.
pub struct HpfSprite {
    pub width: u16,
    pub height: u16,
    /// Raw 8-bit palette-indexed pixels, row-major, `width * height` bytes.
    pub pixels: Vec<u8>,
}

#[derive(Debug)]
pub enum HpfError {
    /// File is too short to contain the minimum required data.
    TooShort,
    /// Header declares zero or implausible dimensions.
    InvalidDimensions { width: u16, height: u16 },
    /// The compressed bitstream ended before producing enough output.
    UnexpectedEndOfStream,
    /// Decompressed pixel count doesn't match width * height.
    SizeMismatch { expected: usize, got: usize },
}

impl fmt::Display for HpfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HpfError::TooShort => write!(f, "HPF file too short"),
            HpfError::InvalidDimensions { width, height } => {
                write!(f, "invalid HPF dimensions: {}x{}", width, height)
            }
            HpfError::UnexpectedEndOfStream => {
                write!(f, "compressed bitstream ended unexpectedly")
            }
            HpfError::SizeMismatch { expected, got } => {
                write!(
                    f,
                    "decompressed {} pixel bytes but header expects {}",
                    got, expected
                )
            }
        }
    }
}

impl std::error::Error for HpfError {}

impl HpfSprite {
    /// Decode an HPF file from its raw bytes.
    ///
    /// If the first 4 bytes match the HPF signature (`0xFF02AA55`), the data
    /// is Huffman-decompressed first. The decompressed (or raw) buffer has an
    /// 8-byte header followed by pixel data.
    pub fn decode(data: &[u8]) -> Result<Self, HpfError> {
        if data.len() < 4 {
            return Err(HpfError::TooShort);
        }

        let signature = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

        let buffer = if signature == HPF_SIGNATURE {
            // Compressed: decompress the bitstream after the 4-byte signature.
            if data.len() < 5 {
                return Err(HpfError::TooShort);
            }
            decompress_huffman(&data[4..])?
        } else {
            // Uncompressed: use as-is.
            data.to_vec()
        };

        // Skip the 8-byte header; width is fixed at 28, height is derived
        // from the remaining data length.
        if buffer.len() <= 8 {
            return Err(HpfError::TooShort);
        }

        let pixel_data = &buffer[8..];
        let width: u16 = 28;
        let height = (pixel_data.len() / width as usize) as u16;

        if height == 0 {
            return Err(HpfError::InvalidDimensions { width, height });
        }

        let pixel_count = width as usize * height as usize;

        Ok(Self {
            width,
            height,
            pixels: pixel_data[..pixel_count].to_vec(),
        })
    }
}

// ---------------------------------------------------------------------------
// Adaptive Huffman decompression
// ---------------------------------------------------------------------------
//
// The codec uses a binary tree with 256 internal nodes (indices 0–255) and
// 257 leaves (indices 256–512):
//
//   - Leaves 256–511 represent byte values 0–255.
//   - Leaf 512 is the end-of-stream sentinel.
//
// The tree is stored implicitly using three arrays:
//
//   - `left_child[256]`  — left  child of internal node i (followed when bit = 0)
//   - `right_child[256]` — right child of internal node i (followed when bit = 1)
//   - `parent[513]`      — parent of any node (internal or leaf)
//
// Initially the tree is a perfect binary tree:
//   node i has left_child = 2i+1, right_child = 2i+2
//
// After emitting each symbol the tree is restructured: the decoded leaf is
// "promoted" toward the root by swapping positions with ancestor nodes. This
// adaptive scheme gives frequently-used symbols shorter codes over time.
//
// Bits are read LSB-first within each byte.

/// Decompress the Huffman bitstream into raw bytes (header + pixels).
fn decompress_huffman(stream: &[u8]) -> Result<Vec<u8>, HpfError> {
    const NODE_COUNT: usize = 256;
    const LEAF_COUNT: usize = 257; // 256 byte values + 1 EOF
    const EOF_LEAF: u16 = (NODE_COUNT + LEAF_COUNT - 1) as u16; // 512

    // Child pointers for the 256 internal nodes.
    // left_child[i]  is followed when the current bit is 0.
    // right_child[i] is followed when the current bit is 1.
    let mut left_child = [0u16; NODE_COUNT];
    let mut right_child = [0u16; NODE_COUNT];

    // Parent mapping for all 513 tree positions (internal nodes + leaves).
    // parent[node] gives the internal node that node is a child of.
    let mut parent = [0u8; NODE_COUNT + LEAF_COUNT];

    // Initialise as a perfect binary tree.
    for i in 0..NODE_COUNT {
        left_child[i] = (2 * i + 1) as u16;
        right_child[i] = (2 * i + 2) as u16;
        parent[2 * i + 1] = i as u8;
        parent[2 * i + 2] = i as u8;
    }

    let mut output = Vec::with_capacity(stream.len() * 2);
    let mut reader = BitReader::new(stream);

    loop {
        // Traverse from root to a leaf.
        let mut node: u16 = 0;
        while node < NODE_COUNT as u16 {
            let bit = reader.read_bit().ok_or(HpfError::UnexpectedEndOfStream)?;
            node = if bit {
                right_child[node as usize]
            } else {
                left_child[node as usize]
            };
        }

        // `node` is now a leaf index (256–512).
        // Restructure the tree before decoding the symbol.
        restructure_tree(&mut left_child, &mut right_child, &mut parent, node);

        // Leaf 512 = end of stream.
        if node == EOF_LEAF {
            break;
        }

        // Leaves 256–511 encode byte values 0–255.
        let byte_value = (node - NODE_COUNT as u16) as u8;
        output.push(byte_value);
    }

    Ok(output)
}

/// Restructure the adaptive Huffman tree after decoding a leaf.
///
/// The decoded `leaf` is "promoted" by repeatedly swapping it with nodes
/// closer to the root. This shortens the code for frequently-occurring
/// symbols.
fn restructure_tree(
    left_child: &mut [u16; 256],
    right_child: &mut [u16; 256],
    parent: &mut [u8; 513],
    leaf: u16,
) {
    let mut current = leaf;
    let mut current_parent = parent[current as usize] as u16;

    // Walk toward the root, swapping at each level.
    while current != 0 && current_parent != 0 {
        let grandparent = parent[current_parent as usize];

        // Determine which child of grandparent points to current_parent,
        // and replace that link with current (promoting current up one level).
        let sibling = left_child[grandparent as usize];
        if sibling == current_parent {
            // current_parent is the left child — swap: grandparent's right becomes current
            let other = right_child[grandparent as usize];
            right_child[grandparent as usize] = current;

            // Fix current_parent's child pointer that pointed to current.
            if left_child[current_parent as usize] == current {
                left_child[current_parent as usize] = other;
            } else {
                right_child[current_parent as usize] = other;
            }

            // Update parent references.
            parent[current as usize] = grandparent;
            parent[other as usize] = current_parent as u8;
        } else {
            // current_parent is the right child — swap: grandparent's left becomes current
            left_child[grandparent as usize] = current;

            // Fix current_parent's child pointer that pointed to current.
            if left_child[current_parent as usize] == current {
                left_child[current_parent as usize] = sibling;
            } else {
                right_child[current_parent as usize] = sibling;
            }

            parent[current as usize] = grandparent;
            parent[sibling as usize] = current_parent as u8;
        }

        // Move up: current becomes the grandparent, and we look at its parent.
        current = grandparent as u16;
        current_parent = parent[current as usize] as u16;
    }
}

// ---------------------------------------------------------------------------
// Bit reader — reads bits LSB-first within each byte
// ---------------------------------------------------------------------------

struct BitReader<'a> {
    data: &'a [u8],
    byte_index: usize,
    bit_index: u8, // 0–7, 0 = LSB
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_index: 0,
            bit_index: 0,
        }
    }

    /// Read the next bit. Returns `None` if the stream is exhausted.
    fn read_bit(&mut self) -> Option<bool> {
        if self.byte_index >= self.data.len() {
            return None;
        }
        let bit = (self.data[self.byte_index] >> self.bit_index) & 1;
        self.bit_index += 1;
        if self.bit_index == 8 {
            self.bit_index = 0;
            self.byte_index += 1;
        }
        Some(bit != 0)
    }
}
