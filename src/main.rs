use clap::Parser;
use rcgen::{
    BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose, PKCS_ECDSA_P384_SHA384,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, process::Command};
use thiserror::Error;

#[derive(Serialize, Deserialize)]
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
        /// Set Subject Alternate Names (example: localhost,google.com,postgres)
        #[arg(long, value_delimiter = ',')]
        sans: Vec<String>,
    },
}

fn main() -> Result<(), String> {
    pre_main()?;
    Ok(())
}

fn pre_main() -> Result<(), Error> {
    let home = home_dir();
    let config_parent = format!("{home}/.config/mkcert-rs");
    let config_path = format!("{home}/.config/mkcert-rs/config.json");

    if !Path::new(&config_path).exists() {
        fs::create_dir_all(config_parent)?;
        fs::write(&config_path, serde_json::to_string(&Config::default())?)?
    };

    let config: Config = serde_json::from_str(&fs::read_to_string(config_path)?)?;

    match Cli::parse() {
        Cli::Install => config.install(),
        Cli::Uninstall => config.uninstall(),
        Cli::New { cert, key, sans } => config.new_cert(cert, key, sans),
    }?;

    Ok(())
}

impl Config {
    fn install(self) -> Result<(), Error> {
        let private_key = KeyPair::generate_for(&PKCS_ECDSA_P384_SHA384)?;

        let mut cert = CertificateParams::default();

        cert.distinguished_name.push(
            DnType::CommonName,
            self.common_name.clone().unwrap_or_default(),
        );
        cert.distinguished_name.push(
            DnType::LocalityName,
            self.locality.clone().unwrap_or_default(),
        );
        cert.distinguished_name.push(
            DnType::CountryName,
            self.country.clone().unwrap_or_default(),
        );
        cert.distinguished_name.push(
            DnType::OrganizationName,
            self.org_name.clone().unwrap_or_default(),
        );
        cert.distinguished_name.push(
            DnType::OrganizationalUnitName,
            self.org_unit.clone().unwrap_or_default(),
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
        let path = format!("{home}/Library/Application Support/mkcert-rs");
        fs::create_dir_all(&path)?;

        let root_cert_path = format!("{path}/rootCA.crt");
        let root_key_path = format!("{path}/rootCA.key");

        // Write to rootCA.crt and rootCA.key
        fs::write(&root_cert_path, ca_cert.pem().as_bytes())?;
        fs::write(&root_key_path, private_key.serialize_pem().as_bytes())?;
        println!("Created certificates in {}", path);

        let command = Command::new("security")
            .arg("add-trusted-cert")
            .arg("-k")
            .arg(format!("{home}/Library/Keychains/login.keychain-db"))
            .arg(&root_cert_path)
            .output()?;

        if command.status.success() {
            println!("Added certificates to the system trust store");
        } else {
            let err_msg = format!("Error: {:#?}", command);
            eprintln!("{err_msg}");
            return Err(Error::Cert(err_msg));
        }

        // If the machine has openssl installed, the tool will also create rootCA.p12
        // This is for manual importing into Firefox
        let command = Command::new("openssl")
            .arg("pkcs12")
            .arg("-export")
            .arg("-in")
            .arg(root_cert_path)
            .arg("-inkey")
            .arg(root_key_path)
            .arg("-out")
            .arg(format!("{path}/rootCA.p12"))
            .output();

        if let Ok(command) = command {
            if command.status.success() {
                println!("Created rootCA.p12 in {}", path);
            }
        }

        Ok(())
    }

    fn uninstall(self) -> Result<(), Error> {
        let home = home_dir();
        let path = format!("{home}/Library/Application Support/mkcert-rs");

        let command = Command::new("security")
            .arg("delete-certificate")
            .arg("-c")
            .arg(self.common_name.clone().unwrap_or_default())
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

        fs::remove_dir_all(path)?;
        println!("Removed certificates from /Application Support/mkcert-rs");
        Ok(())
    }

    fn new_cert(self, cert_name: String, key_name: String, sans: Vec<String>) -> Result<(), Error> {
        let path = format!("{}/Library/Application Support/mkcert-rs", home_dir());
        let root_cert_path = format!("{path}/rootCA.crt");
        let root_key_path = format!("{path}/rootCA.key");

        let root_key =
            KeyPair::from_pem(std::str::from_utf8(fs::read(root_key_path)?.as_slice()).unwrap())?;

        let root_cert = CertificateParams::from_ca_cert_pem(
            std::str::from_utf8(fs::read(root_cert_path)?.as_slice()).unwrap(),
        )?
        .self_signed(&root_key)?;

        let new_key = KeyPair::generate_for(&PKCS_ECDSA_P384_SHA384)?;
        let mut new_certificate = CertificateParams::new(sans)?;

        new_certificate.distinguished_name.push(
            DnType::CommonName,
            self.common_name.clone().unwrap_or_default(),
        );
        new_certificate.distinguished_name.push(
            DnType::LocalityName,
            self.locality.clone().unwrap_or_default(),
        );
        new_certificate.distinguished_name.push(
            DnType::CountryName,
            self.country.clone().unwrap_or_default(),
        );
        new_certificate.distinguished_name.push(
            DnType::OrganizationName,
            self.org_name.clone().unwrap_or_default(),
        );
        new_certificate.distinguished_name.push(
            DnType::OrganizationalUnitName,
            self.org_unit.clone().unwrap_or_default(),
        );

        let new_certificate = new_certificate.signed_by(&new_key, &root_cert, &root_key)?;

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
}

pub fn home_dir() -> String {
    std::env::var_os("HOME")
        .expect("No HOME environment variable set")
        .into_string()
        .expect("Invalid HOME environment variable")
}
