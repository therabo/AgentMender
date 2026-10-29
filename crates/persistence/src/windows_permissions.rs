//! Windows owner-only access via safe Windows ACL wrappers.
use std::{io, path::Path};
use windows_permissions::{
    constants::{AceType, SeObjectType, SecurityInformation},
    utilities::current_process_sid,
    wrappers::{
        ConvertSecurityDescriptorToStringSecurityDescriptor,
        ConvertStringSecurityDescriptorToSecurityDescriptor, EqualSid, GetNamedSecurityInfo,
        SetNamedSecurityInfo,
    },
};
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "owner-only Windows ACL required",
    )
}
pub(crate) fn secure_new_root(path: &Path) -> io::Result<()> {
    let owner = current_process_sid()?;
    let sddl = format!("D:P(A;OICI;FA;;;{owner})");
    let sd = ConvertStringSecurityDescriptorToSecurityDescriptor(&sddl)?;
    SetNamedSecurityInfo(
        path,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        None,
        None,
        sd.dacl(),
        None,
    )?;
    validate(path)
}
pub(crate) fn validate(path: &Path) -> io::Result<()> {
    let sd = GetNamedSecurityInfo(
        path,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl,
    )?;
    let owner = sd.owner().ok_or_else(invalid)?;
    let current = current_process_sid()?;
    if !EqualSid(owner, &current) {
        return Err(invalid());
    }
    let acl = sd.dacl().ok_or_else(invalid)?;
    if acl.len() != 1 {
        return Err(invalid());
    }
    let ace = acl.get_ace(0).ok_or_else(invalid)?;
    let sid = ace.sid().ok_or_else(invalid)?;
    if ace.ace_type() != AceType::ACCESS_ALLOWED_ACE_TYPE
        || !EqualSid(owner, sid)
        || ace.mask().bits() != 0x1f01ff
    {
        return Err(invalid());
    }
    let flags = ace.flags().bits();
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        if flags != 3
            || !ConvertSecurityDescriptorToStringSecurityDescriptor(&sd, SecurityInformation::Dacl)?
                .to_string_lossy()
                .contains("D:P")
        {
            return Err(invalid());
        }
    } else if flags != 0x10 {
        return Err(invalid());
    }
    Ok(())
}
