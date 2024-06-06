# `mkcert-rs`
I tried rewritting `mkcert` but in Rust, using mainly the `rcgen` library that `rustls` provides <br>
This tool generates certificates and key, deriving from a self-signed root CA (which this tool also provides), making development that needs TLS testing much easier

<u>NOTE</u>: Only macOS supported, since it uses macOS's Application Support path, and using the login keychain store

# Usage

Check out the [`config.sample.json`](./config.sample.json) file to see which are the options you can adjust to your likings. <br>
However you don't have to supply anything as there are defaults. <br>
Here are the defaults:
```json
{
  "common_name": "Mkcert Development Certificate",
  "locality": "San Francisco",
  "country": "US",
  "org_unit": "Development",
  "org_name": "Mkcert"
}
```
Rename from `config.sample.json` to `config.json`, and place the file at `$HOME/.config/mkcert-rs/config.json`<br>


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