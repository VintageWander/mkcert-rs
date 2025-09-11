mod config;
use clap::Parser;

use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, PKCS_ECDSA_P384_SHA384,
};
use sha1::{Digest, Sha1};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    process::Command,
};
use thiserror::Error;

use crate::config::{get_config_path, Config};

#[derive(Debug, Error)]
enum Error {
    #[error("Cannot parse config file")]
    Config(#[from] serde_json::Error),
    #[error("Rcgen error: {0:#?}")]
    Rcgen(#[from] rcgen::Error),
    #[error("IO error: {0:#?}")]
    Io(#[from] std::io::Error),
    #[error("Failed to add certificate to the system trust store")]
    Cert(String),
    #[error("Could not get home directory")]
    NoHomeDir,
}

impl From<Error> for String {
    fn from(e: Error) -> Self {
        e.to_string()
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
enum Cli {
    /// Install the certificate authority to the system trust store
    InstallCa,
    /// Remove the certificate authority from the system trust store
    UninstallCa,
    /// Create a new certificate, signed by certificate authority
    New {
        /// Rename the new certificate (example: localhost.crt).
        #[arg(long, default_value = "server.crt")]
        cert: String,
        /// Rename the new private key (example: localhost.key).
        #[arg(long, default_value = "server.key")]
        key: String,
        /// Set Subject Alternate Names (example: localhost,google.com,postgres)
        #[arg(long, value_delimiter = ',')]
        sans: Vec<String>,
    },
}

fn main() -> Result<(), String> {
    match Cli::parse() {
        Cli::InstallCa => install_ca(),
        Cli::UninstallCa => uninstall_ca(),
        Cli::New { cert, key, sans } => new_cert(cert, key, sans),
    }?;

    Ok(())
}

fn install_ca() -> Result<(), Error> {
    let config = Config::read_config()?;

    let mut cert_params = CertificateParams::default();

    cert_params.distinguished_name.push(
        DnType::CommonName,
        config.common_name.clone().unwrap_or_default(),
    );
    cert_params.distinguished_name.push(
        DnType::LocalityName,
        config.locality.clone().unwrap_or_default(),
    );
    cert_params.distinguished_name.push(
        DnType::CountryName,
        config.country.clone().unwrap_or_default(),
    );
    cert_params.distinguished_name.push(
        DnType::OrganizationName,
        config.org_name.clone().unwrap_or_default(),
    );
    cert_params.distinguished_name.push(
        DnType::OrganizationalUnitName,
        config.org_unit.clone().unwrap_or_default(),
    );
    cert_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    cert_params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
        KeyUsagePurpose::KeyCertSign,
    ];
    cert_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let private_key = KeyPair::generate_for(&PKCS_ECDSA_P384_SHA384)?;

    let ca_cert = cert_params.self_signed(&private_key)?;

    let path = get_config_path()?;

    let root_cert_path = path.join("rootCA.crt");
    let root_key_path = path.join("rootCA.key");

    std::fs::write(&root_cert_path, ca_cert.pem().as_bytes())?;
    std::fs::write(&root_key_path, private_key.serialize_pem().as_bytes())?;

    println!("Created certificates in {}", path.display());

    #[cfg(target_os = "macos")]
    let command = {
        let home = dirs::home_dir().unwrap();
        let home = home.to_str().unwrap();
        Command::new("security")
            .arg("add-trusted-cert")
            .arg("-k")
            .arg(format!("{home}/Library/Keychains/login.keychain-db"))
            .arg(&root_cert_path)
            .output()?
    };

    #[cfg(target_os = "windows")]
    let command = Command::new("certutil")
        .arg("-addstore")
        .arg("Root")
        .arg(&root_cert_path)
        .output()?;

    if command.status.success() {
        println!("Added certificates to the system trust store");
    } else {
        let err_msg = format!("Error: {:#?}", command);
        eprintln!("{err_msg}");
        return Err(Error::Cert(err_msg));
    }

    let mut hasher = Sha1::new();
    hasher.update(ca_cert.der());
    let thumbprint_bytes = hasher.finalize();
    let thumbprint = format!("{:X}", thumbprint_bytes);
    Config::write_config(&Config {
        thumbprint: Some(thumbprint),
        ..config
    })?;

    Ok(())
}

fn uninstall_ca() -> Result<(), Error> {
    let config = Config::read_config()?;

    let thumbprint = config.thumbprint.as_ref().ok_or_else(|| {
        Error::Cert(
            "CA thumbprint not found in config. Cannot uninstall. Was the CA ever installed?"
                .to_string(),
        )
    })?;

    #[cfg(target_os = "macos")]
    let command = Command::new("security")
        .arg("delete-certificate")
        .arg("-Z")
        .arg(thumbprint)
        .output()?;

    #[cfg(target_os = "windows")]
    let command = Command::new("certutil")
        .arg("-delstore")
        .arg("Root")
        .arg(thumbprint)
        .output()?;

    if command.status.success() {
        println!("Removed certificates from the system trust store");
    } else {
        let err_msg = format!("Error: {:#?}", command);
        eprintln!("{err_msg}");
        return Err(Error::Cert(err_msg));
    }

    let path = get_config_path()?;
    std::fs::remove_dir_all(path)?;
    println!("Removed certificates");

    Config::write_config(&Config {
        thumbprint: None,
        ..config
    })?;
    Ok(())
}

fn new_cert(cert_name: String, key_name: String, sans: Vec<String>) -> Result<(), Error> {
    let config = Config::read_config()?;

    let path = get_config_path()?;

    let root_cert_path = path.join("rootCA.crt");
    let root_key_path = path.join("rootCA.key");

    let mut root_key_file = OpenOptions::new().read(true).open(&root_key_path)?;
    let mut root_key_str = String::new();
    root_key_file.read_to_string(&mut root_key_str)?;

    let root_key = KeyPair::from_pem(&root_key_str)?;

    let mut root_cert_file = OpenOptions::new().read(true).open(&root_cert_path)?;
    let mut root_cert_str = String::new();
    root_cert_file.read_to_string(&mut root_cert_str)?;

    let root_cert = Issuer::from_ca_cert_pem(&root_cert_str, root_key)?;

    let new_key = KeyPair::generate_for(&PKCS_ECDSA_P384_SHA384)?;
    let mut new_certificate = CertificateParams::new(sans)?;

    new_certificate.distinguished_name.push(
        DnType::CommonName,
        config.common_name.clone().unwrap_or_default(),
    );
    new_certificate.distinguished_name.push(
        DnType::LocalityName,
        config.locality.clone().unwrap_or_default(),
    );
    new_certificate.distinguished_name.push(
        DnType::CountryName,
        config.country.clone().unwrap_or_default(),
    );
    new_certificate.distinguished_name.push(
        DnType::OrganizationName,
        config.org_name.clone().unwrap_or_default(),
    );
    new_certificate.distinguished_name.push(
        DnType::OrganizationalUnitName,
        config.org_unit.clone().unwrap_or_default(),
    );

    let new_certificate = new_certificate.signed_by(&new_key, &root_cert)?;

    let path = std::env::current_dir()?;

    let cert_path = path.join(&cert_name);
    let key_path = path.join(&key_name);

    let mut cert_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&cert_path)?;

    let mut key_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&key_path)?;

    cert_file.write_all(new_certificate.pem().as_bytes())?;
    key_file.write_all(new_key.serialize_pem().as_bytes())?;

    println!("Created new certificate in {}", cert_path.display());

    Ok(())
}
