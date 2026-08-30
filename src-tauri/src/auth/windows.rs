use crate::auth::secret_store::SecretStore;
use sha2::Digest;
use zeroize::Zeroizing;
pub struct CredentialManagerStore;

/// DPAPI fallback store. Files contain only CryptProtectData ciphertext and are replaced
/// atomically by the caller; the entropy binds records to this application namespace.
#[cfg(target_os = "windows")]
pub struct DpapiStore {
    pub directory: std::path::PathBuf,
}
#[cfg(target_os = "windows")]
impl DpapiStore {
    pub fn new(directory: std::path::PathBuf) -> Self {
        Self { directory }
    }
}
#[cfg(target_os = "windows")]
impl SecretStore for DpapiStore {
    fn put(&mut self, name: &str, secret: Zeroizing<String>) -> Result<(), String> {
        use windows_sys::Win32::Security::Cryptography::*;
        std::fs::create_dir_all(&self.directory)
            .map_err(|_| "cannot create secure directory".to_string())?;
        let input = CRYPT_INTEGER_BLOB {
            cbData: secret.len() as u32,
            pbData: secret.as_ptr() as *mut u8,
        };
        let entropy = b"UsageTracker:anthropic:v1";
        let ent = CRYPT_INTEGER_BLOB {
            cbData: entropy.len() as u32,
            pbData: entropy.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        if unsafe {
            CryptProtectData(
                &input,
                std::ptr::null(),
                &ent,
                std::ptr::null(),
                std::ptr::null(),
                0,
                &mut out,
            )
        } == 0
        {
            return Err("DPAPI encryption failed".into());
        }
        let data = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
        unsafe { windows_sys::Win32::Foundation::LocalFree(out.pbData as *mut core::ffi::c_void) };
        let path = self.directory.join(format!(
            "{}.bin",
            sha2::Sha256::digest(name.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, data).map_err(|_| "DPAPI write failed".to_string())?;
        std::fs::rename(tmp, path).map_err(|_| "DPAPI atomic replace failed".to_string())
    }
    fn get(&self, name: &str) -> Result<Option<Zeroizing<String>>, String> {
        use windows_sys::Win32::Security::Cryptography::*;
        let path = self.directory.join(format!(
            "{}.bin",
            sha2::Sha256::digest(name.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));
        let data = match std::fs::read(path) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("DPAPI read failed".into()),
        };
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let entropy = b"UsageTracker:anthropic:v1";
        let ent = CRYPT_INTEGER_BLOB {
            cbData: entropy.len() as u32,
            pbData: entropy.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        if unsafe {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                &ent,
                std::ptr::null(),
                std::ptr::null(),
                0,
                &mut out,
            )
        } == 0
        {
            return Err("DPAPI decryption failed".into());
        }
        let value = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
        unsafe { windows_sys::Win32::Foundation::LocalFree(out.pbData as *mut core::ffi::c_void) };
        String::from_utf8(value)
            .map(Zeroizing::new)
            .map(Some)
            .map_err(|_| "DPAPI value is invalid UTF-8".into())
    }
    fn delete(&mut self, name: &str) -> Result<(), String> {
        let path = self.directory.join(format!(
            "{}.bin",
            sha2::Sha256::digest(name.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("DPAPI delete failed".into()),
        }
    }
}

#[cfg(target_os = "windows")]
impl SecretStore for CredentialManagerStore {
    fn put(&mut self, name: &str, secret: Zeroizing<String>) -> Result<(), String> {
        use windows_sys::Win32::Security::Credentials::*;
        let target = wide(name)?;
        let mut bytes = secret.as_bytes().to_vec();
        let c = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_ptr() as *mut _,
            CredentialBlobSize: bytes.len().try_into().map_err(|_| "credential too large")?,
            CredentialBlob: bytes.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        let ok = unsafe { CredWriteW(&c, 0) };
        bytes.fill(0);
        if ok == 0 {
            return Err(format!("credential manager write failed: {}", unsafe {
                windows_sys::Win32::Foundation::GetLastError()
            }));
        }
        Ok(())
    }
    fn get(&self, name: &str) -> Result<Option<Zeroizing<String>>, String> {
        use windows_sys::Win32::Security::Credentials::*;
        let target = wide(name)?;
        let mut p = std::ptr::null_mut();
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut p) };
        if ok == 0 {
            let e = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if e == 1168 {
                return Ok(None);
            }
            return Err(format!("credential manager read failed: {e}"));
        }
        let b = unsafe {
            std::slice::from_raw_parts((*p).CredentialBlob, (*p).CredentialBlobSize as usize)
        };
        let r = String::from_utf8(b.to_vec())
            .map(Zeroizing::new)
            .map_err(|_| "credential is not UTF-8".to_string());
        unsafe { CredFree(p as *const _) };
        r.map(Some)
    }
    fn delete(&mut self, name: &str) -> Result<(), String> {
        use windows_sys::Win32::Security::Credentials::*;
        let t = wide(name)?;
        let ok = unsafe { CredDeleteW(t.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            let e = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if e == 1168 {
                return Ok(());
            }
            return Err(format!("credential manager delete failed: {e}"));
        }
        Ok(())
    }
}
#[cfg(target_os = "windows")]
fn wide(v: &str) -> Result<Vec<u16>, String> {
    if v.encode_utf16().any(|c| c == 0) {
        return Err("credential name contains NUL".into());
    }
    Ok(v.encode_utf16().chain(std::iter::once(0)).collect())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn dpapi_fallback_can_replace_an_existing_secret() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = DpapiStore::new(directory.path().to_path_buf());
        store
            .put("target", Zeroizing::new("first-value".into()))
            .unwrap();
        store
            .put("target", Zeroizing::new("second-value".into()))
            .unwrap();
        assert_eq!(&*store.get("target").unwrap().unwrap(), "second-value");
    }
}
#[cfg(not(target_os = "windows"))]
impl SecretStore for CredentialManagerStore {
    fn put(&mut self, _: &str, _: Zeroizing<String>) -> Result<(), String> {
        Err("Credential Manager unavailable on this platform".into())
    }
    fn get(&self, _: &str) -> Result<Option<Zeroizing<String>>, String> {
        Err("Credential Manager unavailable on this platform".into())
    }
    fn delete(&mut self, _: &str) -> Result<(), String> {
        Err("Credential Manager unavailable on this platform".into())
    }
}
