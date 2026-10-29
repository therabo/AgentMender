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
fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, reason)
}
pub(crate) fn secure_new_root(path: &Path) -> io::Result<()> {
    let owner = current_process_sid()?;
    let sddl = format!("D:P(A;OICI;FA;;;{owner})");
    let sd = ConvertStringSecurityDescriptorToSecurityDescriptor(&sddl)?;
    SetNamedSecurityInfo(
        path,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner | SecurityInformation::Dacl | SecurityInformation::ProtectedDacl,
        Some(&owner),
        None,
        sd.dacl(),
        None,
    )?;
    validate(path)
}
/// Windows children inherit the DACL, but their owner comes from the process
/// token's default owner (which can be Administrators). Called only for files
/// atomically created empty by this capability, before any content is written.
pub(crate) fn own_new_file(path: &Path) -> io::Result<()> {
    let owner = current_process_sid()?;
    SetNamedSecurityInfo(
        path,
        SeObjectType::SE_FILE_OBJECT,
        SecurityInformation::Owner,
        Some(&owner),
        None,
        None,
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
    let owner = sd
        .owner()
        .ok_or_else(|| invalid("Windows object has no owner"))?;
    let current = current_process_sid()?;
    if !EqualSid(owner, &current) {
        return Err(invalid("Windows object owner differs from current user"));
    }
    let acl = sd
        .dacl()
        .ok_or_else(|| invalid("Windows object has no DACL"))?;
    if acl.len() != 1 {
        return Err(invalid("Windows DACL must contain exactly one owner ACE"));
    }
    let ace = acl
        .get_ace(0)
        .ok_or_else(|| invalid("Windows owner ACE missing"))?;
    let sid = ace
        .sid()
        .ok_or_else(|| invalid("Windows owner ACE SID missing"))?;
    if ace.ace_type() != AceType::ACCESS_ALLOWED_ACE_TYPE
        || !EqualSid(owner, sid)
        || ace.mask().bits() != 0x1f01ff
    {
        return Err(invalid(
            "Windows ACE must grant only the owner full file access",
        ));
    }
    let flags = ace.flags().bits();
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        if flags != 3
            || !ConvertSecurityDescriptorToStringSecurityDescriptor(&sd, SecurityInformation::Dacl)?
                .to_string_lossy()
                .contains("D:P")
        {
            return Err(invalid(
                "Windows root DACL must be protected and inheritable",
            ));
        }
    } else if flags != 0x10 {
        return Err(invalid("Windows file ACE must be inherited"));
    }
    Ok(())
}
