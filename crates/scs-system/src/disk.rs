use std::path::Path;

use rustix::fs::statvfs;
use scs_core::DiskSpace;

use crate::monitor::SystemError;

pub fn probe_disk(path: &Path) -> Result<DiskSpace, SystemError> {
    let stat = statvfs(path).map_err(|err| SystemError::Disk(err.to_string()))?;
    Ok(DiskSpace::from_blocks(
        stat.f_frsize,
        stat.f_blocks,
        stat.f_bavail,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_has_nonzero_capacity() {
        let space = probe_disk(Path::new("/")).unwrap();
        assert!(space.total_bytes > 0);
        assert!(space.available_bytes <= space.total_bytes);
    }
}
