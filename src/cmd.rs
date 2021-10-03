use clap::{Arg, App as ClapApp, ArgMatches, SubCommand};

//File Args
pub const CONFIG_FILE_ARG_NAME: &str = "config yml file";
pub const SCENARIOS_FILE_ARG_NAME: &str = "scenarios yml file";
pub const ENVIRONMENT_FILE_ARG_NAME: &str = "environments yml file";
pub const DATA_FILE_ARG_NAME: &str = "data csv file";
pub const REPORT_FILE_ARG_NAME: &str = "report file";

//Hub
pub const HUB_ADDRESS_ARG_NAME: &str = "hub adress as <ip>::<port>";

//Ports
pub const SERVER_PORT_ARG_NAME: &str = "server port";
pub const SOCKET_PORT_ARG_NAME: &str = "websocket port";

pub const DEFAULT_REPORT_FILE: &str = "report.csv";

pub struct App<'a> {
    arg_matches: ArgMatches<'a>
}

impl<'a> App<'a> {
    pub fn new() -> Self {  
         App {
            arg_matches: create_cmd_app().get_matches()
        }
    }

    pub fn subcommand(&self) -> String {
        self.arg_matches.subcommand().0.to_string()
    }

    pub fn arg_value_as_str(&self, arg: &str) -> String {
        match self.arg_matches.subcommand().1 {
            Some(x) => match x.value_of(arg) {
                Some(y) => y.to_string(),
                None => "".to_string()
            },
            None => "".to_string()
        }
    }

    pub fn arg_value_as_u16(&self, arg: &str) -> u16 {
        match self.arg_matches.subcommand().1 {
            Some(x) => match x.value_of(arg) {
                Some(y) => y.parse::<u16>().unwrap(),
                None => 0
            },
            None => 0
        }
    }
}

fn create_cmd_app<'a, 'b>() -> ClapApp<'a, 'b> {
    ClapApp::new("Bombardier")
        .version("0.1.0")
        .author("Coding Yogi <aniket.g2185@gmail.com>")
        .subcommand(SubCommand::with_name("bombard")
                .about("Executes the test")
                .args(&[
                    get_arg(CONFIG_FILE_ARG_NAME, "c", true, "Execution config yml file")
                    .validator(is_yml)
                    .display_order(0),

                    get_arg(SCENARIOS_FILE_ARG_NAME, "s", true, "Scenarios yml file")
                    .validator(is_yml)
                    .display_order(1),

                    get_arg(ENVIRONMENT_FILE_ARG_NAME, "e", false, "Environment yml file")
                    .validator(is_yml)
                    .display_order(2),

                    get_arg(DATA_FILE_ARG_NAME, "d", false, "Data csv file")
                    .validator(is_csv)
                    .display_order(3),

                    get_arg(REPORT_FILE_ARG_NAME, "r", false, "report csv file")
                    .validator(is_csv)
                    .display_order(4),
                ]))

        .subcommand(SubCommand::with_name("report")
                .about("Generates the report from report file")
                .arg(get_arg(REPORT_FILE_ARG_NAME, "r", true, "report file")
                .validator(is_csv)))

        .subcommand(SubCommand::with_name("node")
                .about("Starts bombardier as a node")
                .arg(get_arg(HUB_ADDRESS_ARG_NAME, "h", true, "hub address <ip>:<port>")))

        .subcommand(SubCommand::with_name("hub")
                .about("Starts bombardier as a hub server")
                .args(&[
                    get_arg(SERVER_PORT_ARG_NAME, "p", true, "rest server port")
                    .validator(is_u16),

                    get_arg(SOCKET_PORT_ARG_NAME, "s", true, "socket server port")
                    .validator(is_u16)
                ])
        )
}

fn is_yml(file_path: String) -> Result<(),String> {
    match file_path.ends_with(".yml") ||file_path.ends_with(".yaml") {
        true => Ok(()),
        false => Err(String::from("File should be a .yml file"))
    }
}

fn is_csv(file_path: String)-> Result<(),String> {
    match file_path.ends_with(".csv") {
        true => Ok(()),
        false => Err(String::from("File should be a .csv file"))
    }
}

fn is_u16(value: String)-> Result<(),String> {
    match value.parse::<u16>() {
        Ok(_) => Ok(()),
        Err(_) => Err(String::from("should be an integer"))
    }
}

fn get_arg(name: &'static str, short:&str, required: bool, help: &'static str) -> Arg<'static, 'static> {
    Arg::with_name(name)
        .short(short)
        .takes_value(true)
        .required(required)
        .help(help)
}

pub fn arg_value_as_str(matches: Option<&ArgMatches>, arg: &str) -> String {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.to_string(),
                        None => "".to_string()
        },
        None => "".to_string()
    }
}

pub fn arg_value_as_u16(matches: Option<&ArgMatches>, arg: &str) -> u16 {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.parse::<u16>().unwrap(),
                        None => 0
        },
        None => 0
    }
}