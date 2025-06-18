use anyhow::Context;
use std::io;

pub(crate) fn is_pna<R: io::Read>(mut reader: R) -> anyhow::Result<bool> {
    let mut buf = [0u8; pna::PNA_HEADER.len()];
    reader
        .read_exact(&mut buf)
        .with_context(|| "failed to read archive header")?;
    Ok(buf == *pna::PNA_HEADER)
}
