# `mkcert-rs`
I tried rewritting `mkcert` but in Rust, using mainly the `rcgen` library that `rustls` provides <br>
This tool generates certificates and key, deriving from a self-signed root CA (which this tool also provides), making development that needs TLS testing much easier

<u>NOTE</u>: Only macOS supported, since it uses macOS's Application Support path, and using the login keychain store<br>
<u>NOTE</u>: Do make sure that you have `openssl` installed, since the certificate will also export in `.p12` format and import it to the certificate trust store

# Usage

Run the command `mkcert-rs` for the first time for the tool to create the initial configuration file at `$HOME/.config/mkcert-rs/config.json`<br>
The config file looks like this: <br>
```json
{
  "common_name": "Mkcert Development Certificate",
  "locality": "San Francisco",
  "country": "US",
  "org_unit": "Development",
  "org_name": "Mkcert"
}
```

You can adjust it to your likings

After that, run `mkcert-rs install`, the tool will
- Create `rootCA.crt` and `rootCA.key` into `$HOME/Library/Application Support/mkcert-rs`
- Install them into `$HOME/Library/Keychains/login.keychain-db`, which is the `login` keychain in the `Keychain Access` app

If you have `openssl` installed, the tool will automatically create `rootCA.p12` key, type in your encryption passphrase and the command will complete

The `.p12` file can be used to manually add into your `Firefox` browser certificates

## Options
```
Usage: mkcert-rs <COMMAND>

Commands:
  install    Install the certificate to the system trust store
  uninstall  Remove the certificate from the system trust store
  new        Create a new certificate
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

# Troubleshooting
## `Firefox`
Currently I haven't figured out a way to make `Firefox` works out of the box, you have to import the `rootCA.p12` cert manually into Firefox<br>
