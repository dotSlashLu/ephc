use clap::{App, Arg};

const DEFAULT_REFRESH_INTERVAL: &str = "1";
const DEFAULT_PROBE_INTERVAL: &str = "1000";
const DEFAULT_CONNECT_TIMEOUT: &str = "100";
const DEFAULT_RESTORE: &str = "3";
const DEFAULT_REMOVE: &str = "3";

#[derive(Debug, Clone)]
pub struct AppOpt {
    pub allow_list: Option<Vec<String>>,
    pub block_list: Option<Vec<String>>,
    pub refresh_interval: u64,
    pub probe_interval: u64,
    pub connection_timeout: u64,
    pub restore: u32,
    pub remove: u32,
    pub cluster_name: Option<String>,
    pub alert_channel: Option<String>,
}

pub(crate) fn init() -> AppOpt {
    env_logger::init();

    let matches = App::new("ephc")
        .version("0.1")
        .author("pan1c <qiang@pan1c.org>")
        .about("Endpoint health check for Kubernetes")
        .arg(
            Arg::with_name("allow_list")
                .short("a")
                .long("allow")
                .required(false)
                .value_name("ALLOW")
                .multiple(true)
                .takes_value(true)
                .help("Only do health check for these services"),
        )
        .arg(
            Arg::with_name("block_list")
                .short("b")
                .long("block")
                .value_name("BLOCK")
                .required(false)
                .multiple(true)
                .takes_value(true)
                .help("Do health check for all services except these"),
        )
        .arg(
            Arg::with_name("refresh_interval")
                .short("i")
                .long("refresh_interval")
                .value_name("REFRESH_INTERVAL")
                .required(false)
                .takes_value(true)
                .default_value(DEFAULT_REFRESH_INTERVAL)
                .help("Interval in seconds to refresh service from k8s"),
        )
        .arg(
            Arg::with_name("probe_interval")
                .short("p")
                .long("probe_interval")
                .value_name("PROBE_INTERVAL")
                .required(false)
                .takes_value(true)
                .default_value(DEFAULT_PROBE_INTERVAL)
                .help("Interval in milliseconds to probe services"),
        )
        .arg(
            Arg::with_name("connection_timeout")
                .short("t")
                .long("connection_timeout")
                .value_name("CONNECTION_TIMEOUT")
                .required(false)
                .takes_value(true)
                .default_value(DEFAULT_CONNECT_TIMEOUT)
                .help("Connection timeout in millisecond"),
        )
        .arg(
            Arg::with_name("remove")
                .short("r")
                .long("remove")
                .value_name("REMOVE")
                .required(false)
                .takes_value(true)
                .default_value(DEFAULT_REMOVE)
                .help("How many times an endpoint failed probing should be removed"),
        )
        .arg(
            Arg::with_name("restore")
                .short("u")
                .long("restore")
                .value_name("RESTORE")
                .required(false)
                .takes_value(true)
                .default_value(DEFAULT_RESTORE)
                .help("How many times an endpoint removed successfully probed should be restored"),
        )
        .arg(
            Arg::with_name("cluster_name")
                .short("C")
                .long("cluster")
                .value_name("CLUSTER")
                .required(false)
                .takes_value(true)
                .help("cluster name of this kubernetes used with alert"),
        )
        .arg(
            Arg::with_name("alert")
                .short("A")
                .long("alert")
                .value_name("ALERT")
                .required(false)
                .multiple(true)
                .takes_value(true)
                .help(
                    "alert webhook url, in the form of scheme://url, \
                    for example: wecom://https://exmaple.com, \
                    supported channels:
                        - wecom(and compatibles)",
                ),
        )
        .get_matches();

    let allow_list_values = matches.values_of("allow_list");
    let allow_list: Option<Vec<String>> = match allow_list_values {
        Some(values) => Some(values.map(|el| el.to_owned()).collect()),
        None => None,
    };

    let block_list_values = matches.values_of("block_list");
    let block_list: Option<Vec<String>> = match block_list_values {
        Some(values) => Some(values.map(|el| el.to_owned()).collect()),
        None => None,
    };

    let refresh_interval: u64 = match matches.value_of("refresh_interval") {
        Some(i) => i.parse().unwrap(),
        None => DEFAULT_REFRESH_INTERVAL.parse().unwrap(),
    };

    let probe_interval: u64 = match matches.value_of("probe_interval") {
        Some(i) => i.parse().unwrap(),
        None => DEFAULT_PROBE_INTERVAL.parse().unwrap(),
    };

    let connection_timeout: u64 = match matches.value_of("connection_timeout") {
        Some(i) => i.parse().unwrap(),
        None => DEFAULT_CONNECT_TIMEOUT.parse().unwrap(),
    };

    let restore: u32 = match matches.value_of("restore") {
        Some(i) => i.parse().unwrap(),
        None => DEFAULT_RESTORE.parse().unwrap(),
    };

    let remove: u32 = match matches.value_of("remove") {
        Some(i) => i.parse().unwrap(),
        None => DEFAULT_REMOVE.parse().unwrap(),
    };

    let cluster_name: Option<String> = match matches.value_of("cluster_name") {
        Some(i) => Some(i.to_owned()),
        None => None,
    };

    let alert_channel: Option<String> = match matches.value_of("alert") {
        Some(i) => Some(i.to_owned()),
        None => None,
    };

    AppOpt {
        allow_list,
        block_list,
        refresh_interval,
        probe_interval,
        connection_timeout,
        restore,
        remove,
        cluster_name,
        alert_channel,
    }
}
