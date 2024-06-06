use clap::Parser;
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose, PKCS_ECDSA_P256_SHA256,
};
use serde::Deserialize;
use std::{fs, process::Command};
use thiserror::Error;

#[derive(Deserialize)]
struct Config {
    common_name: Option<String>,
    locality: Option<String>,
    country: Option<String>,
    org_unit: Option<String>,
    org_name: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            common_name: Some("Mkcert Development Certificate".into()),
            locality: Some("San Francisco".into()),
            country: Some("US".into()),
            org_unit: Some("Development".into()),
            org_name: Some("Mkcert".into()),
        }
    }
}

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
    pre_main()?;
    Ok(())
}

fn pre_main() -> Result<(), Error> {
    let config: Config = serde_json::from_str(&fs::read_to_string("./config.json")?)?;
    let cli = Cli::parse();
    match cli {
        Cli::Install => install(&config),
        Cli::Uninstall => uninstall(&config),
        Cli::New { cert, key, sans } => new_cert(cert, key, sans),
    }?;
    Ok(())
}

fn install(
    Config {
        common_name,
        locality,
        country,
        org_unit,
        org_name,
    }: &Config,
) -> Result<(), Error> {
    let private_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;

    let mut cert = CertificateParams::default();

    cert.distinguished_name
        .push(DnType::CommonName, common_name.clone().unwrap_or_default());
    cert.distinguished_name
        .push(DnType::LocalityName, locality.clone().unwrap_or_default());
    cert.distinguished_name
        .push(DnType::CountryName, country.clone().unwrap_or_default());
    cert.distinguished_name.push(
        DnType::OrganizationName,
        org_name.clone().unwrap_or_default(),
    );
    cert.distinguished_name.push(
        DnType::OrganizationalUnitName,
        org_unit.clone().unwrap_or_default(),
    );
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

fn uninstall(Config { common_name, .. }: &Config) -> Result<(), Error> {
    let home = home_dir();
    let path = format!("{home}/Library/Application Support/mkcert-rs/");

    let root_cert_path = format!("{path}/rootCA.crt");
    let root_key_path = format!("{path}/rootCA.key");

    let command = Command::new("security")
        .arg("delete-certificate")
        .arg("-c")
        .arg(common_name.clone().unwrap_or_default())
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

pub fn home_dir() -> String {
    std::env::var_os("HOME")
        .expect("No HOME environment variable set")
        .into_string()
        .expect("Invalid HOME environment variable")
}
