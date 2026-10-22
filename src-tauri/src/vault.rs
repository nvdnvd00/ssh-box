const SERVICE_NAME: &str = "com.darien.sshbox";

pub fn set_host_secret(host_id: &str, secret: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_id).map_err(|e| e.to_string())?;
    entry.set_password(secret).map_err(|e| e.to_string())
}

pub fn get_host_secret(host_id: &str) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_id).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn delete_host_secret(host_id: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_id).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn set_secret(host_id: &str, secret: &str) -> Result<(), String> {
    set_host_secret(host_id, secret)
}

pub fn get_secret(host_id: &str) -> Result<Option<String>, String> {
    get_host_secret(host_id)
}

pub fn delete_secret(host_id: &str) -> Result<(), String> {
    delete_host_secret(host_id)
}
