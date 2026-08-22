use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};

use crate::process::{ProcessRunner, RunOptions, command_exists};

const REQUIRED: &[&str] = &[
    "ca.crt",
    "ca.key",
    "server.crt",
    "server.key",
    "client.crt",
    "client.key",
    "client.pk8.key",
];

pub fn issue_certificates(
    root: &Path,
    output: Option<&Path>,
    runner: &dyn ProcessRunner,
) -> Result<String> {
    let destination = absolute_output(root, output)?;
    if REQUIRED.iter().all(|file| destination.join(file).is_file()) {
        return Ok(format!(
            "certificates already present in {}",
            destination.display()
        ));
    }
    if destination.exists() {
        bail!(
            "{} is incomplete; remove it explicitly before reissuing certificates",
            destination.display()
        );
    }
    if !command_exists(runner, "openssl") {
        bail!("OpenSSL is required to issue the local TLS fixture certificates");
    }

    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("{} has no parent directory", destination.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("could not create {}", parent.display()))?;
    let temporary = create_temporary_directory(parent)?;
    let operation = issue_into(&temporary, runner).and_then(|()| {
        fs::rename(&temporary, &destination).with_context(|| {
            format!(
                "could not move issued certificates to {}",
                destination.display()
            )
        })
    });
    if operation.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    operation?;
    Ok(format!(
        "issued local certificates in {}",
        destination.display()
    ))
}

fn issue_into(directory: &Path, runner: &dyn ProcessRunner) -> Result<()> {
    openssl(
        runner,
        directory,
        &[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "ca.key",
            "-out",
            "ca.crt",
            "-days",
            "825",
            "-subj",
            "/CN=Irodori Samples Local CA",
        ],
    )?;
    openssl(
        runner,
        directory,
        &[
            "req",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "server.key",
            "-out",
            "server.csr",
            "-subj",
            "/CN=localhost",
        ],
    )?;
    fs::write(
        directory.join("server.ext"),
        "subjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth\n",
    )?;
    openssl(
        runner,
        directory,
        &[
            "x509",
            "-req",
            "-in",
            "server.csr",
            "-CA",
            "ca.crt",
            "-CAkey",
            "ca.key",
            "-CAcreateserial",
            "-out",
            "server.crt",
            "-days",
            "825",
            "-extfile",
            "server.ext",
        ],
    )?;
    openssl(
        runner,
        directory,
        &[
            "req",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "client.key",
            "-out",
            "client.csr",
            "-subj",
            "/CN=irodori_cert",
        ],
    )?;
    fs::write(
        directory.join("client.ext"),
        "extendedKeyUsage=clientAuth\n",
    )?;
    openssl(
        runner,
        directory,
        &[
            "x509",
            "-req",
            "-in",
            "client.csr",
            "-CA",
            "ca.crt",
            "-CAkey",
            "ca.key",
            "-CAcreateserial",
            "-out",
            "client.crt",
            "-days",
            "825",
            "-extfile",
            "client.ext",
        ],
    )?;
    openssl(
        runner,
        directory,
        &[
            "pkcs8",
            "-topk8",
            "-nocrypt",
            "-in",
            "client.key",
            "-out",
            "client.pk8.key",
        ],
    )?;

    set_private_permissions(directory)?;
    for file in ["server.csr", "client.csr", "server.ext", "client.ext"] {
        remove_if_present(&directory.join(file))?;
    }
    for file in REQUIRED {
        if !directory.join(file).is_file() {
            bail!("OpenSSL did not create required certificate file {file}");
        }
    }
    Ok(())
}

fn openssl(runner: &dyn ProcessRunner, cwd: &Path, args: &[&str]) -> Result<()> {
    runner.run(
        "openssl",
        &args
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
        RunOptions::cwd(cwd),
    )?;
    Ok(())
}

#[cfg(unix)]
fn set_private_permissions(directory: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for file in ["ca.key", "server.key", "client.key", "client.pk8.key"] {
        fs::set_permissions(directory.join(file), fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_private_permissions(_directory: &Path) -> Result<()> {
    Ok(())
}

fn absolute_output(root: &Path, output: Option<&Path>) -> Result<PathBuf> {
    match output {
        None => Ok(root.join("tls/certs")),
        Some(path) if path.is_absolute() => Ok(path.to_path_buf()),
        Some(path) => Ok(std::env::current_dir()?.join(path)),
    }
}

fn create_temporary_directory(parent: &Path) -> Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for attempt in 0..100 {
        let path = parent.join(format!(
            ".certs-{}-{timestamp}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    bail!("could not allocate a temporary certificate directory")
}

fn remove_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::{ProcessOutput, RunOptions};

    struct NeverRunner;
    impl ProcessRunner for NeverRunner {
        fn run(&self, _: &str, _: &[String], _: RunOptions) -> Result<ProcessOutput> {
            panic!("runner should not be called")
        }
    }

    #[test]
    fn complete_certificate_directory_is_reused() {
        let root = std::env::temp_dir().join(format!(
            "irodori-certs-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let destination = root.join("tls/certs");
        fs::create_dir_all(&destination).unwrap();
        for file in REQUIRED {
            fs::write(destination.join(file), b"fixture").unwrap();
        }
        let message = issue_certificates(&root, None, &NeverRunner).unwrap();
        assert!(message.contains("already present"));
        fs::remove_dir_all(root).unwrap();
    }
}
