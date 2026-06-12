# FMI get temperature

`fmi_get_temp` fetches the latest temperature reading from the Finnish
Meteorological Institute (FMI) open data API for a configured station ID
(`fmisid`). It parses the FMI WFS XML response, prints the most recent
temperature to stdout, and can optionally forward the reading with MQTT and/or
CoAP.

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

Enable CoAP publishing:

```sh
cargo run -- --coap-enabled --coap-url coap://localhost/store --coap-key temperature
```

CLI options:

```
Usage: fmi_get_temp [OPTIONS]

Options:
  -v, --verbose
  -d, --debug
  -t, --trace
      --fmi-url <FMI_URL>              [default: http://opendata.fmi.fi/wfs/fin?service=WFS&version=2.0.0&request=GetFeature&storedquery_id=fmi::observations::weather::timevaluepair&parameters=t2m&fmisid=###FMI_SID###&starttime=###START_TIME###]
      --fmi-sid <FMI_SID>              [default: 101118]
      --fmi-mins <FMI_MINS>            [default: 60]
      --mqtt-enabled
      --mqtt-host <MQTT_HOST>          [default: localhost]
      --mqtt-port <MQTT_PORT>          [default: 1883]
      --mqtt-topic <MQTT_TOPIC>        [default: fmi_temp/101118]
      --mqtt-username <MQTT_USERNAME>
      --mqtt-password <MQTT_PASSWORD>
      --coap-enabled
      --coap-url <COAP_URL>            [default: coap://localhost/store]
      --coap-key <COAP_KEY>            [default: temperature]
  -h, --help                           Print help
```

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

Check dependency status with:

```sh
cargo outdated --workspace --root-deps-only
cargo update --dry-run
```

See [AGENTS.md](AGENTS.md) for contributor guidelines.
