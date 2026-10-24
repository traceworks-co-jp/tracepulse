use crate::error::AppError;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chacha20poly1305::{
    ChaCha20Poly1305, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use rand::{RngCore, rngs::OsRng};
use std::{fs, io::Write, path::Path};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

pub const PREFIX: &str = "tpenc:v1:";
pub const MASK: &str = "********";
const WRAP_CONTEXT: &[u8] = b"TracePulse/envelope/v1/data-key";

#[cfg(test)]
thread_local! {
    pub(crate) static CRYPTO_OPERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct MasterKey([u8; 32]);

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("MasterKey([REDACTED])")
    }
}

fn crypto_error(message: &str) -> AppError {
    AppError::Config(format!("credential protection: {message}"))
}

impl MasterKey {
    pub fn generate() -> Result<Self, AppError> {
        let mut key = Self([0; 32]);
        OsRng
            .try_fill_bytes(&mut key.0)
            .map_err(|_| crypto_error("secure random generation failed"))?;
        Ok(key)
    }

    pub fn parse(encoded: &str) -> Result<Self, AppError> {
        let bytes = if encoded.len() == 64 && encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            let mut bytes = Zeroizing::new(Vec::with_capacity(32));
            for offset in (0..64).step_by(2) {
                let digit = |byte: u8| match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    b'A'..=b'F' => byte - b'A' + 10,
                    _ => unreachable!("hex validated"),
                };
                bytes.push(
                    digit(encoded.as_bytes()[offset]) * 16 + digit(encoded.as_bytes()[offset + 1]),
                );
            }
            bytes
        } else {
            Zeroizing::new(STANDARD.decode(encoded)
                .map_err(|_| crypto_error("TRACEPULSE_MASTER_KEY must be 64 hex characters or Base64 encoding of 32 bytes"))?)
        };
        let key: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| crypto_error("master key must contain exactly 32 bytes"))?;
        Ok(Self(key))
    }

    pub fn load(directory: &Path, allow_create: bool) -> Result<Self, AppError> {
        match std::env::var("TRACEPULSE_MASTER_KEY") {
            Ok(value) => return Self::parse(&Zeroizing::new(value)),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(crypto_error("TRACEPULSE_MASTER_KEY is not valid text"));
            }
            Err(std::env::VarError::NotPresent) => {}
        }
        Self::load_file(directory, allow_create)
    }

    fn load_file(directory: &Path, allow_create: bool) -> Result<Self, AppError> {
        let directory = if directory.as_os_str().is_empty() {
            Path::new(".")
        } else {
            directory
        };
        let path = directory.join("secret.key");
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.file_type().is_file() {
                    return Err(crypto_error(
                        "secret.key must be a regular file, not a symlink",
                    ));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if metadata.permissions().mode() & 0o777 != 0o600 {
                        return Err(crypto_error("secret.key must have Unix permissions 0600"));
                    }
                }
                restrict_file(&path)?;
                let bytes = Zeroizing::new(fs::read(path)?);
                let key = bytes.as_slice().try_into().map_err(|_| {
                    crypto_error(
                        "secret.key must contain exactly 32 raw bytes; refusing to replace it",
                    )
                })?;
                Ok(Self(key))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if !allow_create {
                    return Err(crypto_error(
                        "master key is missing for encrypted data; restore secret.key or supply TRACEPULSE_MASTER_KEY",
                    ));
                }
                fs::create_dir_all(directory)?;
                let key = Self::generate()?;
                let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
                restrict_file(temporary.path())?;
                temporary.write_all(&key.0)?;
                temporary.as_file().sync_all()?;
                match temporary.persist_noclobber(&path) {
                    Ok(_) => {
                        sync_directory(directory)?;
                        Ok(key)
                    }
                    Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                        Self::load_file(directory, false)
                    }
                    Err(error) => Err(error.error.into()),
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn encrypt(&self, plaintext: &[u8], context: &[u8]) -> Result<String, AppError> {
        #[cfg(test)]
        CRYPTO_OPERATIONS.with(|count| count.set(count.get() + 1));
        let data_key = Self::generate()?;
        let mut wrap_nonce = [0; 12];
        let mut value_nonce = [0; 12];
        OsRng
            .try_fill_bytes(&mut wrap_nonce)
            .and_then(|()| OsRng.try_fill_bytes(&mut value_nonce))
            .map_err(|_| crypto_error("secure random generation failed"))?;
        let wrapped = ChaCha20Poly1305::new((&self.0).into())
            .encrypt(
                Nonce::from_slice(&wrap_nonce),
                Payload {
                    msg: &data_key.0,
                    aad: WRAP_CONTEXT,
                },
            )
            .map_err(|_| crypto_error("data-key wrapping failed"))?;
        let ciphertext = ChaCha20Poly1305::new((&data_key.0).into())
            .encrypt(
                Nonce::from_slice(&value_nonce),
                Payload {
                    msg: plaintext,
                    aad: context,
                },
            )
            .map_err(|_| crypto_error("encryption failed"))?;
        let mut envelope = Vec::with_capacity(72 + ciphertext.len());
        envelope.extend_from_slice(&wrap_nonce);
        envelope.extend_from_slice(&wrapped);
        envelope.extend_from_slice(&value_nonce);
        envelope.extend_from_slice(&ciphertext);
        Ok(format!("{PREFIX}{}", STANDARD.encode(envelope)))
    }

    pub fn decrypt(&self, envelope: &str, context: &[u8]) -> Result<Zeroizing<Vec<u8>>, AppError> {
        #[cfg(test)]
        CRYPTO_OPERATIONS.with(|count| count.set(count.get() + 1));
        let encoded = envelope.strip_prefix(PREFIX).ok_or_else(|| {
            crypto_error("unsupported or plaintext credential; migration is required")
        })?;
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| crypto_error("invalid credential envelope"))?;
        if bytes.len() < 88 {
            return Err(crypto_error("truncated credential envelope"));
        }
        let data_key = Zeroizing::new(
            ChaCha20Poly1305::new((&self.0).into())
                .decrypt(
                    Nonce::from_slice(&bytes[..12]),
                    Payload {
                        msg: &bytes[12..60],
                        aad: WRAP_CONTEXT,
                    },
                )
                .map_err(|_| {
                    crypto_error("master key mismatch or corrupted credential envelope")
                })?,
        );
        let cipher = ChaCha20Poly1305::new_from_slice(&data_key)
            .map_err(|_| crypto_error("invalid wrapped data key"))?;
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&bytes[60..72]),
                Payload {
                    msg: &bytes[72..],
                    aad: context,
                },
            )
            .map_err(|_| crypto_error("credential authentication failed"))?;
        Ok(Zeroizing::new(plaintext))
    }

    pub fn decrypt_string(&self, envelope: &str, context: &[u8]) -> Result<String, AppError> {
        String::from_utf8(self.decrypt(envelope, context)?.to_vec())
            .map_err(|_| crypto_error("credential is not valid UTF-8"))
    }
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let mut random = [0; 16];
    OsRng
        .try_fill_bytes(&mut random)
        .map_err(|_| crypto_error("secure random generation failed"))?;
    let salt = SaltString::encode_b64(&random).map_err(|_| crypto_error("salt encoding failed"))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| crypto_error("password hashing failed"))
}

pub fn verify_password(password: &str, encoded: &str) -> Result<bool, AppError> {
    let hash =
        PasswordHash::new(encoded).map_err(|_| crypto_error("invalid stored password hash"))?;
    if hash.algorithm.as_str() != "argon2id" || hash.version != Some(19) {
        return Err(crypto_error("stored password hash must use Argon2id v19"));
    }
    match Argon2::default().verify_password(password.as_bytes(), &hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(_) => Err(crypto_error("password verification failed")),
    }
}

pub fn write_private_atomic(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    restrict_file(temporary.path())?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| AppError::from(error.error))?;
    sync_directory(parent)?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    fs::File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(not(windows))]
fn restrict_file(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

#[cfg(windows)]
fn restrict_file(path: &Path) -> Result<(), AppError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
            DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, SetFileSecurityW,
        },
    };
    let descriptor: Vec<u16> = "D:P(A;;FA;;;OW)\0".encode_utf16().collect();
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut security = std::ptr::null_mut();
    // Apply an owner-only protected DACL before any secret bytes are written.
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            descriptor.as_ptr(),
            1,
            &mut security,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(crypto_error(
                "cannot construct owner-only Windows file permissions",
            ));
        }

        let result = SetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            security,
        );
        LocalFree(security);
        if result == 0 {
            return Err(crypto_error(
                "cannot apply owner-only Windows file permissions",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_roundtrips_with_independent_keys_nonces_and_authenticated_context() {
        let key = MasterKey::generate().unwrap();
        for plaintext in [
            b"".as_slice(),
            b"private-community",
            "日本語の資格情報".as_bytes(),
        ] {
            let first = key.encrypt(plaintext, b"device/one/community").unwrap();
            let second = key.encrypt(plaintext, b"device/one/community").unwrap();
            assert_ne!(first, second);
            assert_eq!(
                key.decrypt(&first, b"device/one/community")
                    .unwrap()
                    .as_slice(),
                plaintext
            );
            assert!(key.decrypt(&first, b"device/two/community").is_err());
            assert!(
                MasterKey::generate()
                    .unwrap()
                    .decrypt(&first, b"device/one/community")
                    .is_err()
            );
            let bytes = STANDARD
                .decode(first.strip_prefix(PREFIX).unwrap())
                .unwrap();
            for offset in [0, 12, 59, 60, 71, bytes.len() - 1] {
                let mut corrupted = bytes.clone();
                corrupted[offset] ^= 1;
                assert!(
                    key.decrypt(
                        &format!("{PREFIX}{}", STANDARD.encode(corrupted)),
                        b"device/one/community"
                    )
                    .is_err()
                );
            }
        }
        for value in [
            "plaintext",
            "tpenc:v2:invalid",
            "tpenc:v1:bad!",
            "tpenc:v1:AA==",
        ] {
            assert!(key.decrypt(value, b"context").is_err());
        }
        assert!(!format!("{key:?}").contains(&STANDARD.encode(key.0)));
    }

    #[test]
    fn password_hashes_are_argon2id_salted_and_verifiable() {
        let first = hash_password("日本語-password").unwrap();
        let second = hash_password("日本語-password").unwrap();
        assert!(first.starts_with("$argon2id$v=19$"));
        assert_ne!(first, second);
        assert!(verify_password("日本語-password", &first).unwrap());
        assert!(!verify_password("wrong", &first).unwrap());
        assert!(verify_password("anything", "not-a-hash").is_err());
        assert!(
            verify_password(
                "anything",
                "$argon2i$v=19$m=19456,t=2,p=1$c2FsdHNhbHQ$dGVzdA"
            )
            .is_err()
        );
    }

    #[test]
    fn key_file_persists_and_missing_or_malformed_keys_are_never_replaced() {
        let directory = tempfile::tempdir().unwrap();
        assert!(MasterKey::load_file(directory.path(), false).is_err());
        assert!(!directory.path().join("secret.key").exists());
        let first = MasterKey::load_file(directory.path(), true).unwrap();
        let second = MasterKey::load_file(directory.path(), false).unwrap();
        assert!(first.0 == second.0);
        assert_eq!(
            fs::read(directory.path().join("secret.key")).unwrap().len(),
            32
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file = directory.path().join("secret.key");
            assert_eq!(
                fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600
            );
            fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
            assert!(MasterKey::load_file(directory.path(), true).is_err());
            fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        }
        fs::write(directory.path().join("secret.key"), b"bad").unwrap();
        assert!(MasterKey::load_file(directory.path(), true).is_err());
        assert_eq!(
            fs::read(directory.path().join("secret.key")).unwrap(),
            b"bad"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_key_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("other"), [1; 32]).unwrap();
        std::os::unix::fs::symlink(
            directory.path().join("other"),
            directory.path().join("secret.key"),
        )
        .unwrap();
        assert!(MasterKey::load_file(directory.path(), true).is_err());
    }

    #[test]
    fn environment_override_is_validated_in_isolated_processes() {
        let binary = std::env::current_exe().unwrap();
        for value in [
            "42".repeat(32),
            STANDARD.encode([0x42; 32]),
            "invalid-key".into(),
            String::new(),
        ] {
            let output = std::process::Command::new(&binary)
                .args([
                    "--exact",
                    "crypto::tests::environment_override_child",
                    "--nocapture",
                ])
                .env("TRACEPULSE_KEY_TEST", "1")
                .env("TRACEPULSE_MASTER_KEY", &value)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        }
    }

    #[test]
    fn environment_override_child() {
        if std::env::var_os("TRACEPULSE_KEY_TEST").is_none() {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("secret.key"), b"malformed-file-key").unwrap();
        let encoded = std::env::var("TRACEPULSE_MASTER_KEY").unwrap();
        let result = MasterKey::load(directory.path(), false);
        if encoded == "invalid-key" || encoded.is_empty() {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().0 == [0x42; 32]);
        }
        assert_eq!(
            fs::read(directory.path().join("secret.key")).unwrap(),
            b"malformed-file-key"
        );
    }
}
