# FMI get temperature

The program fetches the latest temperature readings from Ilmatieteen Laitos (FMI) at given location (by fmisid)
and parses the received XML.

It then prints the most recent temperature reading to stdout if no other options are given.

Additionally, it can send the data with MQTT publish message and/or CoAP POST request
if either or both of those options are enabled.

A list of stations can be found here with fmisid values:

<https://www.ilmatieteenlaitos.fi/havaintoasemat>

Example values:

* Helsinki-Vantaa lentoasema: fmisid 100968
* Pirkkala lentoasema: fmisid 101118
* Vaasa lentoasema: fmisid 101462
* Oulu lentoasema: fmisid 101786
* Rovaniemi lentoasema: fmisid 101920
* Kittilä lentoasema: fmisid 101986
* Salla Naruska: fmisid 101966

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
