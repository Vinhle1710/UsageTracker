#[cfg(target_os = "windows")]
pub struct CredentialManagerStore;
#[cfg(not(target_os = "windows"))]
pub struct CredentialManagerStore;
