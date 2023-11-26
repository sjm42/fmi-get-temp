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

USAGE:
    fmi_get_temp [FLAGS] [OPTIONS]

FLAGS:
        --coap-enabled    
    -d, --debug           
    -h, --help            Prints help information
        --mqtt-enabled    
    -t, --trace           
    -V, --version         Prints version information
    -v, --verbose         

OPTIONS:
        --coap-key <coap-key>         [default: temperature]
        --coap-url <coap-url>         [default: coap://localhost/store]
        --fmi-sid <fmi-sid>           [default: 101118]
        --fmi-url <fmi-url>           [default:
                                     http://opendata.fmi.fi/wfs/fin?service=WFS&version=2.0.0&request=GetFeature&storedquery_id=fmi::observations::weather::timevaluepair&parameters=t2m&fmisid=###FMI_SID###&starttime=###START_TIME###]
        --mqtt-host <mqtt-host>       [default: localhost]
        --mqtt-port <mqtt-port>       [default: 1883]
        --mqtt-topic <mqtt-topic>     [default: fmi_temp/101118]

```
