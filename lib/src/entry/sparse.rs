//! Sparse file extent metadata.

use std::io;

/// A data-bearing region of a sparse file's logical address space.
///
/// Zero-sized regions are valid. They consume no payload bytes and are kept
/// when parsing so non-canonical but unambiguous archives can round-trip.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DataRegion {
    offset: u64,
    size: u64,
}

impl DataRegion {
    /// Creates a data region at `offset` with the given byte `size`.
    #[inline]
    pub const fn new(offset: u64, size: u64) -> Self {
        Self { offset, size }
    }

    /// Returns the logical byte offset where this region begins.
    #[inline]
    pub const fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the number of payload bytes in this region.
    #[inline]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Returns the logical end offset, or `None` if it overflows `u64`.
    #[inline]
    pub const fn checked_end(&self) -> Option<u64> {
        self.offset.checked_add(self.size)
    }
}

/// Mapping from compact `FDAT` payload bytes to a file's logical offsets.
///
/// The `SPAR` payload is an 8-byte big-endian logical size followed by zero
/// or more `(offset, size)` pairs, each encoded as two big-endian `u64`s.
/// Data for positive-sized regions appears in `FDAT` in region order. Gaps are
/// filesystem holes. Zero-sized and adjacent regions are valid and preserved.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SparseMap {
    logical_size: u64,
    regions: Vec<DataRegion>,
}

impl SparseMap {
    /// Constructs a validated sparse map without canonicalizing its regions.
    ///
    /// Offsets may repeat when the repeated regions have zero size. Adjacent
    /// positive regions are allowed. Positive-sized regions must not overlap.
    ///
    /// # Errors
    ///
    /// Returns an error when regions are out of order, overlap, overflow, or
    /// extend beyond `logical_size`.
    #[inline]
    pub fn try_new(logical_size: u64, regions: Vec<DataRegion>) -> io::Result<Self> {
        validate_regions(logical_size, &regions)?;
        Ok(Self {
            logical_size,
            regions,
        })
    }

    /// Returns the reconstructed logical file size.
    #[inline]
    pub const fn logical_size(&self) -> u64 {
        self.logical_size
    }

    /// Returns the ordered data-bearing regions of the sparse file.
    #[inline]
    pub fn regions(&self) -> &[DataRegion] {
        &self.regions
    }

    /// Number of decoded payload bytes represented by this map.
    #[inline]
    pub fn data_size(&self) -> Option<u64> {
        self.regions
            .iter()
            .try_fold(0u64, |sum, region| sum.checked_add(region.size))
    }

    /// Returns whether the map contains no positive-sized data regions.
    #[inline]
    pub fn is_all_hole(&self) -> bool {
        self.regions.iter().all(|region| region.size == 0)
    }

    /// Parses and validates a `SPAR` chunk payload.
    ///
    /// # Errors
    ///
    /// Returns an error when the payload length is invalid or the decoded map
    /// violates sparse-region ordering or bounds constraints.
    #[inline]
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        let (logical_size, region_bytes) = data.split_first_chunk::<8>().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid SPAR chunk length")
        })?;
        let (pairs, remainder) = region_bytes.as_chunks::<16>();
        if !remainder.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid SPAR chunk length",
            ));
        }

        let logical_size = u64::from_be_bytes(*logical_size);
        let mut regions = Vec::with_capacity(pairs.len());
        for pair in pairs {
            let offset = u64::from_be_bytes([
                pair[0], pair[1], pair[2], pair[3], pair[4], pair[5], pair[6], pair[7],
            ]);
            let size = u64::from_be_bytes([
                pair[8], pair[9], pair[10], pair[11], pair[12], pair[13], pair[14], pair[15],
            ]);
            regions.push(DataRegion::new(offset, size));
        }
        Self::try_new(logical_size, regions)
    }

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(8 + self.regions.len() * 16);
        data.extend_from_slice(&self.logical_size.to_be_bytes());
        for region in &self.regions {
            data.extend_from_slice(&region.offset.to_be_bytes());
            data.extend_from_slice(&region.size.to_be_bytes());
        }
        data
    }
}

fn validate_regions(logical_size: u64, regions: &[DataRegion]) -> io::Result<()> {
    let mut previous_offset = None;
    let mut previous_data_end = 0u64;
    for region in regions {
        if previous_offset.is_some_and(|offset| region.offset < offset) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SPAR regions are not ordered by offset",
            ));
        }
        previous_offset = Some(region.offset);
        let end = region.checked_end().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "SPAR region offset overflows")
        })?;
        if end > logical_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SPAR region exceeds logical size",
            ));
        }
        if region.size == 0 {
            continue;
        }
        if region.offset < previous_data_end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SPAR data regions overlap",
            ));
        }
        previous_data_end = end;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn preserves_noncanonical_but_unambiguous_regions() {
        let regions = vec![
            DataRegion::new(0, 10),
            DataRegion::new(10, 0),
            DataRegion::new(10, 0),
            DataRegion::new(10, 5),
            DataRegion::new(15, 5),
        ];
        let map = SparseMap::try_new(20, regions.clone()).unwrap();
        assert_eq!(map.regions(), regions);
        assert_eq!(SparseMap::from_bytes(&map.to_bytes()).unwrap(), map);
        assert_eq!(map.data_size(), Some(20));
    }

    #[test]
    fn zero_region_inside_data_region_is_unambiguous() {
        let map =
            SparseMap::try_new(100, vec![DataRegion::new(10, 20), DataRegion::new(20, 0)]).unwrap();
        assert_eq!(map.data_size(), Some(20));
    }

    #[test]
    fn rejects_positive_overlap() {
        assert!(
            SparseMap::try_new(100, vec![DataRegion::new(10, 20), DataRegion::new(20, 5)],)
                .is_err()
        );
    }

    #[test]
    fn rejects_descending_offsets() {
        assert!(
            SparseMap::try_new(100, vec![DataRegion::new(20, 0), DataRegion::new(10, 0)],).is_err()
        );
    }

    #[test]
    fn rejects_region_end_overflow() {
        assert!(SparseMap::try_new(u64::MAX, vec![DataRegion::new(u64::MAX - 1, 2)],).is_err());
    }

    #[test]
    fn rejects_region_past_logical_size() {
        assert!(SparseMap::try_new(9, vec![DataRegion::new(9, 1)]).is_err());
    }

    #[test]
    fn rejects_malformed_wire_length() {
        assert!(SparseMap::from_bytes(&[0; 9]).is_err());
    }
}
