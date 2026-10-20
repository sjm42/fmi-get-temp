// config.rs

use crate::*;

#[derive(Debug, Clone, Parser)]
pub struct OptsCommon {
    #[arg(short, long)]
    pub verbose: bool,
    #[arg(short, long)]
    pub debug: bool,
    #[arg(short, long)]
    pub trace: bool,

    #[arg(
        long,
        default_value = "https://opendata.fmi.fi/wfs/fin?service=WFS&version=2.0.0&request=GetFeature&storedquery_id=fmi::observations::weather::timevaluepair&parameters=t2m&fmisid=###FMI_SID###&starttime=###START_TIME###"
    )]
    pub fmi_url: String,

    // https://www.ilmatieteenlaitos.fi/havaintoasemat
    // Helsinki-Vantaa lentoasema: fmisid 100968
    // Pirkkala lentoasema: fmisid 101118
    // Vaasa lentoasema: fmisid 101462
    // Oulu lentoasema: fmisid 101786
    // Rovaniemi lentoasema: fmisid 101920
    // Kittilä lentoasema: fmisid 101986
    // Salla Naruska: fmisid 101966
    #[arg(long, default_value = "101118")]
    pub fmi_sid: String,

    /// Observation lookback in minutes (must be positive).
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(i64).range(1..))]
    pub fmi_mins: i64,

    /// Maximum acceptable observation age in minutes (independent of the query lookback).
    #[arg(long, default_value_t = 1440, value_parser = clap::value_parser!(i64).range(1..))]
    pub fmi_max_age_mins: i64,

    /// Maximum decompressed FMI response size in bytes.
    #[arg(long, default_value_t = DEFAULT_MAX_BODY_BYTES, value_parser = clap::value_parser!(u64).range(1..))]
    pub fmi_max_body_bytes: u64,

    // MQTT support
    #[arg(long)]
    pub mqtt_enabled: bool,
    #[arg(long, default_value = "localhost")]
    pub mqtt_host: String,
    #[arg(long, default_value_t = 1883)]
    pub mqtt_port: u16,
    /// MQTT topic; defaults to fmi_temp/<fmi-sid> when omitted.
    #[arg(long)]
    pub mqtt_topic: Option<String>,
    #[arg(long, requires = "mqtt_password")]
    pub mqtt_username: Option<String>,
    #[arg(long, requires = "mqtt_username")]
    pub mqtt_password: Option<String>,

    /// Enable MQTT TLS with certificate validation (plaintext is the default).
    #[arg(long)]
    pub mqtt_tls: bool,
    /// PEM CA bundle for MQTT TLS; otherwise use system trust roots.
    #[arg(long, requires = "mqtt_tls")]
    pub mqtt_tls_ca: Option<PathBuf>,
    /// MQTT client ID; otherwise generate a unique ID per invocation.
    #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new())]
    pub mqtt_client_id: Option<String>,
    /// Deadline in seconds for the complete MQTT operation.
    #[arg(long, default_value_t = 15, value_parser = clap::value_parser!(u64).range(1..))]
    pub mqtt_timeout_secs: u64,

    // CoAP support
    #[arg(long)]
    pub coap_enabled: bool,
    #[arg(long, default_value = "coap://localhost/store")]
    pub coap_url: String,
    #[arg(long, default_value = "temperature")]
    pub coap_key: String,
}

impl OptsCommon {
    pub fn mqtt_topic(&self) -> Cow<'_, str> {
        match &self.mqtt_topic {
            Some(topic) => Cow::Borrowed(topic),
            None => Cow::Owned(format!("fmi_temp/{}", self.fmi_sid)),
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.fmi_sid.parse::<u32>().is_ok_and(|id| id > 0),
            "FMI station ID must be a positive integer"
        );
        ensure!(self.fmi_mins > 0, "FMI lookback must be positive");
        let duration = Duration::try_minutes(self.fmi_mins).context("FMI lookback is too large")?;
        Utc::now()
            .checked_sub_signed(duration)
            .context("FMI lookback exceeds the supported date range")?;
        ensure!(self.fmi_max_age_mins > 0, "Maximum observation age must be positive");
        Duration::try_minutes(self.fmi_max_age_mins).context("Maximum observation age is too large")?;
        ensure!(self.fmi_max_body_bytes > 0, "FMI response size limit must be positive");
        if self.mqtt_enabled {
            self.validate_mqtt()?;
        }
        Ok(())
    }

    pub(crate) fn validate_mqtt(&self) -> anyhow::Result<()> {
        ensure!(
            self.mqtt_username.is_some() == self.mqtt_password.is_some(),
            "MQTT username and password must be supplied together"
        );
        if let Some(username) = &self.mqtt_username {
            validate_mqtt_string("username", username)?;
        }
        if let Some(password) = &self.mqtt_password {
            ensure!(password.len() <= u16::MAX as usize, "MQTT password exceeds 65535 bytes");
        }
        let topic = self.mqtt_topic();
        validate_mqtt_string("topic", &topic)?;
        ensure!(
            !topic.contains(['+', '#']),
            "MQTT publish topic must not contain wildcards"
        );
        ensure!(self.mqtt_timeout_secs > 0, "MQTT timeout must be positive");
        ensure!(
            self.mqtt_tls_ca.is_none() || self.mqtt_tls,
            "MQTT CA bundle requires TLS"
        );
        if let Some(id) = &self.mqtt_client_id {
            validate_mqtt_string("client ID", id)?;
        }
        Ok(())
    }

    pub fn get_loglevel(&self) -> Level {
        if self.trace {
            Level::TRACE
        } else if self.debug {
            Level::DEBUG
        } else if self.verbose {
            Level::INFO
        } else {
            Level::ERROR
        }
    }

    pub fn get_log_filter(&self) -> String {
        let dependency_level = if self.verbose || self.debug || self.trace {
            "warn"
        } else {
            "error"
        };
        format!(
            "{dependency_level},fmi_get_temp={}",
            self.get_loglevel().as_str().to_ascii_lowercase()
        )
    }

    pub fn start_pgm(&self, name: &str) -> anyhow::Result<()> {
        let filter = match env::var("RUST_LOG") {
            Ok(value) => EnvFilter::try_new(value).context("Invalid RUST_LOG filter")?,
            Err(env::VarError::NotPresent) => EnvFilter::new(self.get_log_filter()),
            Err(error) => return Err(error).context("Invalid RUST_LOG value"),
        };
        tracing_fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .with_target(false)
            .try_init()
            .map_err(|error| anyhow!("Could not initialize tracing: {error}"))?;

        info!("Starting up {name} v{}...", env!("CARGO_PKG_VERSION"));
        debug!("Git branch: {}", env!("GIT_BRANCH"));
        debug!("Git commit: {}", env!("GIT_COMMIT"));
        debug!("Source timestamp: {}", env!("SOURCE_TIMESTAMP"));
        debug!("Compiler version: {}", env!("RUSTC_VERSION"));
        Ok(())
    }
}

pub(crate) fn validate_mqtt_string(name: &str, value: &str) -> anyhow::Result<()> {
    ensure!(!value.is_empty(), "MQTT {name} must not be empty");
    ensure!(value.len() <= u16::MAX as usize, "MQTT {name} exceeds 65535 bytes");
    ensure!(!value.contains('\0'), "MQTT {name} must not contain NUL characters");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_ranges_and_incomplete_credentials() {
        for args in [
            vec!["fmi_get_temp", "--fmi-mins", "0"],
            vec!["fmi_get_temp", "--fmi-max-age-mins", "0"],
            vec!["fmi_get_temp", "--mqtt-timeout-secs", "0"],
            vec!["fmi_get_temp", "--fmi-max-body-bytes", "0"],
            vec!["fmi_get_temp", "--mqtt-username", "alice"],
            vec!["fmi_get_temp", "--mqtt-password", "secret"],
            vec!["fmi_get_temp", "--mqtt-tls-ca", "ca.pem"],
            vec!["fmi_get_temp", "--mqtt-client-id", ""],
        ] {
            assert!(OptsCommon::try_parse_from(args).is_err());
        }
        for minutes in [i64::MAX, 1_000_000_000_000] {
            let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
            opts.fmi_mins = minutes;
            assert!(opts.validate().is_err());
        }
    }

    #[test]
    fn resolves_default_topic_from_station_and_preserves_explicit_topic() {
        let opts = OptsCommon::parse_from(["fmi_get_temp", "--fmi-sid", "100968"]);
        assert_eq!(opts.mqtt_topic(), "fmi_temp/100968");
        let opts = OptsCommon::parse_from(["fmi_get_temp", "--fmi-sid", "100968", "--mqtt-topic", "fmi_temp/101118"]);
        assert_eq!(opts.mqtt_topic(), "fmi_temp/101118");
    }

    #[test]
    fn validates_mqtt_wire_fields_without_echoing_credentials() {
        for topic in ["", "a/#", "+/a", "a\0b", &"a".repeat(65_536)] {
            let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
            opts.mqtt_topic = Some(topic.to_owned());
            assert!(opts.validate_mqtt().is_err());
        }
        for username in ["", "alice\0", &"a".repeat(65_536)] {
            let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
            opts.mqtt_username = Some(username.to_owned());
            opts.mqtt_password = Some("secret".into());
            assert!(opts.validate_mqtt().is_err());
        }
        let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
        opts.mqtt_username = Some("alice".into());
        opts.mqtt_password = Some("".into());
        assert!(opts.validate_mqtt().is_ok());
        opts.mqtt_password = Some("secret".repeat(11_000));
        let error = opts.validate_mqtt().unwrap_err();
        assert!(!error.to_string().contains("secret"));
        opts.mqtt_password = Some("secret".into());
        for id in ["bad\0id", &"a".repeat(65_536)] {
            opts.mqtt_client_id = Some(id.to_owned());
            assert!(opts.validate_mqtt().is_err());
        }
    }

    #[test]
    fn defaults_preserve_plaintext_mqtt_and_enable_https_for_fmi() {
        let opts = OptsCommon::parse_from(["fmi_get_temp"]);
        opts.validate().unwrap();
        assert!(opts.fmi_url.starts_with("https://"));
        assert!(!opts.mqtt_tls);
        assert_eq!(opts.mqtt_port, 1883);
    }

    #[test]
    fn filters_dependency_debug_logs_independently_of_application_logs() {
        #[derive(Clone)]
        struct LogWriter(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for LogWriter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let opts = OptsCommon::parse_from(["fmi_get_temp", "--debug"]);
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = LogWriter(output.clone());
        let subscriber = tracing_fmt()
            .with_env_filter(EnvFilter::new(opts.get_log_filter()))
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            debug!(target: "fmi_get_temp", "app debug");
            debug!(target: "rumqttc", "dependency debug");
            warn!(target: "rumqttc", "dependency warning");
        });
        let log = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(log.contains("app debug"));
        assert!(!log.contains("dependency debug"));
        assert!(log.contains("dependency warning"));
    }
}
// EOF
