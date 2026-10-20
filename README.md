# FMI get temperature

`fmi_get_temp` fetches the latest temperature reading from the Finnish
Meteorological Institute (FMI) open data API for a configured station ID
(`fmisid`). It parses the FMI WFS XML response, prints the most recent
temperature to stdout, and can optionally forward the reading with MQTT and/or
CoAP.

FMI requests use HTTPS with certificate validation. The newest observation is
selected by its parsed timestamp, including its UTC offset, and must match the
configured station and the `t2m` temperature parameter. Missing or non-finite
temperatures in the newest observation are errors; older readings are not used
as a fallback. Conflicting temperatures at the newest timestamp are also
rejected. Successful output is one temperature followed by a newline.

The newest observation must be at most 24 hours old by default. Configure this
limit with `--fmi-max-age-mins` (default: 1440). Timestamps more than five minutes
in the future are rejected, allowing a small amount of clock skew. These checks
do not expand the `--fmi-mins` query window or enable fallback to older values.

## Station IDs

A list of FMI stations and `fmisid` values is available here:

<https://www.ilmatieteenlaitos.fi/havaintoasemat>

Common examples:

* Helsinki-Vantaa lentoasema: fmisid 100968
* Pirkkala lentoasema: fmisid 101118
* Vaasa lentoasema: fmisid 101462
* Oulu lentoasema: fmisid 101786
* Rovaniemi lentoasema: fmisid 101920
* Kittilä lentoasema: fmisid 101986
* Salla Naruska: fmisid 101966

## Usage

Fetch the default station:

```sh
cargo run -- --fmi-sid 101118
```

Enable MQTT publishing:

```sh
cargo run -- --mqtt-enabled --mqtt-host localhost --mqtt-topic fmi_temp/101118
```

An explicit `--mqtt-topic` is used exactly as supplied. When omitted, the topic
defaults to `fmi_temp/<fmi-sid>`, so `--fmi-sid 100968` publishes to
`fmi_temp/100968` unless a topic is supplied.

MQTT uses plaintext and port 1883 by default. Enable TLS explicitly and select
the broker's TLS port:

```sh
cargo run -- --mqtt-enabled --mqtt-host broker.example --mqtt-tls --mqtt-port 8883
```

TLS validates the broker certificate against system trust roots. Use
`--mqtt-tls-ca /path/to/ca.pem` with `--mqtt-tls` to trust a specific PEM CA
bundle instead. MQTT usernames and passwords must be supplied together. The
username must be nonempty; an explicitly empty password is allowed. MQTT UTF-8
fields reject NUL characters and fields are limited to 65,535 bytes. Publish
topics must be nonempty and cannot contain `+` or `#` wildcards.

Each CLI invocation generates a unique client ID and uses a clean session.
`--mqtt-client-id` overrides the generated ID. Publishing retains the message
with QoS 1 and sends a JSON number, for example `{"temperature":2.5}`.
The complete MQTT operation, including acknowledgement and disconnect, has a
15-second deadline configurable with `--mqtt-timeout-secs`.

Library callers of `mqtt_send` supply the exact client ID through its
`client_id` argument; that argument is never randomized or overridden by
`OptsCommon::mqtt_client_id`. The CLI resolves its option before calling the
library. Client IDs are connection/session identifiers and are not included in
the temperature message.

Enable CoAP publishing:

```sh
cargo run -- --coap-enabled --coap-url coap://localhost/store --coap-key temperature
```

Inspect all CLI options:

```sh
cargo run -- --help
```

`--fmi-mins` defaults to 60 and must be a positive lookback that fits the
supported date range. FMI responses must be UTF-8 XML with a `text/xml`,
`application/xml`, or `text/*+xml` / `application/*+xml` content type. Responses
are limited to 2 MiB of decompressed data by default; change that with
`--fmi-max-body-bytes`. The XML parser also limits documents to 100,000 nodes
and rejects DTDs.

Configuration, FMI fetching/parsing, and MQTT publishing failures return a
nonzero exit status. A fetched temperature is printed before publishing, so it
can still appear on stdout when MQTT publishing fails.

All diagnostics go to stderr. `--verbose`, `--debug`, and `--trace` control
application logging; dependency logging stays at warning level when one of
these flags is set, and otherwise at error level. `RUST_LOG` overrides these
defaults, for example:

```sh
RUST_LOG='warn,fmi_get_temp=debug,rumqttc=info' cargo run -- --mqtt-enabled
```

Invalid `RUST_LOG` filters produce a configuration error. Raw FMI response
bodies are logged only at trace level.

## Development

Useful commands:

```sh
cargo build
cargo build --release
cargo build --profile minsize
cargo test
cargo fmt
cargo clippy --all-targets --all-features
```

Tests use XML fixtures and local mock servers. Timestamp boundary tests use a
fixed clock, and network fixtures are generated with recent timestamps. Network
helpers have deadlines; CLI tests terminate their child processes on timeout.

Check dependency status with:

```sh
cargo outdated --workspace --root-deps-only
cargo update --dry-run
```

See [AGENTS.md](AGENTS.md) for contributor guidelines.
