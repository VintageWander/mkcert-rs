mod env;

use clap::Parser;
use env::{common_name, country, home_dir, locality, org_name, org_unit};
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose, PKCS_ECDSA_P256_SHA256,
};
use std::{fs, process::Command};
use thiserror::Error;

#[derive(Debug, Error)]
enum Error {
    #[error("Rcgen error: {0:#?}")]
    Rcgen(#[from] rcgen::Error),
    #[error("IO error: {0:#?}")]
    Io(#[from] std::io::Error),
    #[error("Failed to add certificate to the system trust store")]
    Cert(String),
}

impl From<Error> for String {
    fn from(e: Error) -> Self {
        e.to_string()
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
enum Cli {
    /// Install the certificate to the system trust store
    Install,
    /// Remove the certificate from the system trust store
    Uninstall,
    /// Create a new certificate
    New {
        /// Rename the new certificate (example: localhost.crt).
        #[arg(long, default_value = "server.crt")]
        cert: String,
        /// Rename the new private key (example: localhost.key).
        #[arg(long, default_value = "server.key")]
        key: String,
        /// Set Subject Alternate Names
        #[arg(long, value_delimiter = ',')]
        sans: Vec<String>,
    },
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    match cli {
        Cli::Install => install(),
        Cli::Uninstall => uninstall(),
        Cli::New { cert, key, sans } => new_cert(cert, key, sans),
    }?;
    Ok(())
}

fn install() -> Result<(), Error> {
    let private_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;

    let mut cert = CertificateParams::default();

    cert.distinguished_name
        .push(DnType::CommonName, common_name());
    cert.distinguished_name
        .push(DnType::LocalityName, locality());
    cert.distinguished_name.push(DnType::CountryName, country());
    cert.distinguished_name
        .push(DnType::OrganizationName, org_name());
    cert.distinguished_name
        .push(DnType::OrganizationalUnitName, org_unit());
    cert.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    cert.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
        KeyUsagePurpose::KeyCertSign,
    ];
    cert.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];

    let ca_cert = cert.self_signed(&private_key)?;

    let home = home_dir();
    let path = format!("{home}/Library/Application Support/mkcert-rs/");
    fs::create_dir_all(&path)?;

    let root_cert_path = format!("{path}/rootCA.crt");
    let root_key_path = format!("{path}/rootCA.key");

    // Write to rootCA.crt and rootCA.key
    fs::write(&root_cert_path, ca_cert.pem().as_bytes())?;
    fs::write(root_key_path, private_key.serialize_pem().as_bytes())?;
    println!("Created certificates in {}", path);

    let command = Command::new("security")
        .arg("add-trusted-cert")
        // .arg("-d")
        .arg("-k")
        .arg(format!("{home}/Library/Keychains/login.keychain-db"))
        .arg(root_cert_path)
        .output()?;

    if command.status.success() {
        println!("Added certificates to the system trust store");
    } else {
        let err_msg = format!("Error: {:#?}", command);
        eprintln!("{err_msg}");
        return Err(Error::Cert(err_msg));
    }

    Ok(())
}

fn uninstall() -> Result<(), Error> {
    let home = home_dir();
    let path = format!("{home}/Library/Application Support/mkcert-rs/");

    let root_cert_path = format!("{path}/rootCA.crt");
    let root_key_path = format!("{path}/rootCA.key");

    let command = Command::new("security")
        .arg("delete-certificate")
        .arg("-c")
        .arg(common_name())
        .arg("-t")
        .arg(format!("{home}/Library/Keychains/login.keychain-db"))
        .output()?;

    if command.status.success() {
        println!("Removed certificates from the system trust store");
    } else {
        let err_msg = format!("Error: {:#?}", command);
        eprintln!("{err_msg}");
        return Err(Error::Cert(err_msg));
    }

    fs::remove_file(&root_cert_path)?;
    fs::remove_file(&root_key_path)?;
    println!("Removed certificates from /Application Support/mkcert-rs");
    Ok(())
}

fn new_cert(cert_name: String, key_name: String, sans: Vec<String>) -> Result<(), Error> {
    let path = format!("{}/Library/Application Support/mkcert-rs/", home_dir());
    let root_cert_path = format!("{path}/rootCA.crt");
    let root_key_path = format!("{path}/rootCA.key");

    let root_key =
        KeyPair::from_pem(std::str::from_utf8(fs::read(root_key_path)?.as_slice()).unwrap())?;

    let root_cert = CertificateParams::from_ca_cert_pem(
        std::str::from_utf8(fs::read(root_cert_path)?.as_slice()).unwrap(),
    )?
    .self_signed(&root_key)?;

    let new_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
    let new_certificate =
        CertificateParams::new(sans)?.signed_by(&new_key, &root_cert, &root_key)?;

    let path = std::env::current_dir()?.to_str().unwrap().to_string();

    fs::write(
        format!("{path}/{cert_name}"),
        new_certificate.pem().as_bytes(),
    )?;
    fs::write(
        format!("{path}/{key_name}"),
        new_key.serialize_pem().as_bytes(),
    )?;
    println!("Created new certificate in {path}");

    Ok(())
}