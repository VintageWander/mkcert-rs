# `mkcert-rs`
I tried rewritting `mkcert` but in Rust, using mainly the `rcgen` library that `rustls` provides <br />
This tool generates certificates and key, deriving from a self-signed root CA (which this tool also provides), making development that needs TLS testing much easier <br />
I've added support for both macOS and Windows

# Usage

Run the command `mkcert-rs` for the first time for the tool to create the initial configuration file at `$HOME/.config/mkcert-rs/config.json`<br>
The config file looks like this: <br>
```json
{
  "common_name": "Mkcert Development CA",
  "locality": "San Francisco",
  "country": "US",
  "org_unit": "Development",
  "org_name": "mkcert-rs"
}
```

You can adjust it to your likings

After that, run `mkcert-rs install-ca`, the tool will
- Create `rootCA.crt` and `rootCA.key` into `$HOME/.config/mkcert-rs`
- macOS: Install them into `$HOME/Library/Keychains/login.keychain-db`, which is the `login` keychain in the `Keychain Access` app
- Windows: Install them into `Trusted Root Certification Authorities/Certificates`

## Options
```
Usage: mkcert-rs <COMMAND>

Commands:
  install-ca    Install the certificate authority to the system trust store
  uninstall-ca  Remove the certificate authority from the system trust store
  new           Create a new certificate, signed by certificate authority
  help          Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
