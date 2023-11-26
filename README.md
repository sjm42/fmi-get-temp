# FMI get temperature

The program prints latest temperature reading to stdout if no options are given.
Additionally, it can spit out the data with MQTT and/or CoAP if requested.

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
