use serde::{Deserialize, Serialize};

/// Offset index for chunks in a flat array or list.
///
/// # Example
///
/// \[0, 3, 5, 7\] contains chunk [0, 3), [3, 5), [5, 7)
///
/// Use [`Self::iter`] to iterate through the chunks.
///
/// ```
/// use bevy_motion_matching::motion_data::chunk::ChunkOffsets;
///
/// let mut offsets = ChunkOffsets::new();
/// offsets.add_chunk(3);
/// offsets.add_chunk(5);
/// offsets.add_chunk(7);
///
/// let mut prev_end = 0;
/// for (start, end) in offsets.iter() {
///     // Start index will always equal previous end index.
///     assert_eq!(start, prev_end);
///     // End index will always be larger than start index.
///     assert!(end > start);
///     prev_end = end;
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChunkOffsets(Vec<usize>);

impl ChunkOffsets {
    /// Initialize offsets with `0` as the first default element.
    pub fn new() -> Self {
        Self(vec![0])
    }

    /// Number of chunks present.
    pub fn num_chunks(&self) -> usize {
        self.0.len().saturating_sub(1)
    }

    pub fn push_chunk(&mut self, chunk_len: usize) {
        self.0.push(self.0[self.num_chunks()] + chunk_len);
    }

    pub fn get_chunk(&self, index: usize) -> Option<(usize, usize)> {
        Some((*self.0.get(index)?, *self.0.get(index + 1)?))
    }

    pub fn get_chunk_unchecked(&self, index: usize) -> (usize, usize) {
        (self.0[index], self.0[index + 1])
    }

    pub fn iter(&self) -> ChunkOffsetsIter<'_> {
        ChunkOffsetsIter {
            offsets: self,
            chunk_index: 0,
        }
    }
}

pub struct ChunkOffsetsIter<'a> {
    offsets: &'a ChunkOffsets,
    chunk_index: usize,
}

impl Iterator for ChunkOffsetsIter<'_> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.offsets.get_chunk(self.chunk_index);
        self.chunk_index += 1;
        chunk
    }
}

impl Default for ChunkOffsets {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ChunkIterator {
    type Item;

    fn offsets(&self) -> &ChunkOffsets;

    fn items(&self) -> &[Self::Item];

    /// Create an iterator that iterates through items `Self::Item` chunk by chunk.
    fn iter_chunk<'a>(&'a self) -> impl Iterator<Item = &'a [Self::Item]>
    where
        Self::Item: 'a,
    {
        self.offsets()
            .iter()
            .map(|(start, end)| &self.items()[start..end])
    }

    /// Create an iterator that iterates through the items `Self::Item` inside a given chunk index.
    fn get_chunk(&self, chunk_index: usize) -> Option<&[Self::Item]> {
        let (start, end) = self.offsets().get_chunk(chunk_index)?;
        Some(&self.items()[start..end])
    }

    /// Unsafe version of the [`ChunkIterator::get_chunk`] method.
    fn get_chunk_unchecked(&self, chunk_index: usize) -> &[Self::Item] {
        let (start, end) = self.offsets().get_chunk_unchecked(chunk_index);
        &self.items()[start..end]
    }
}
