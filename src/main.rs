use colored::*;
use serde::Deserialize;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;
use regex;
use clap_v3::{App, Arg};

#[derive(Debug, Deserialize)]
enum EntryType {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    #[serde(rename = "type")]
    entry_type: EntryType,
    message: String,
    context: String,
    artifact: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    event: Event,
    filename: String,
    line_number: i32,
    timestamp: String,
}

#[derive(Debug, Deserialize)]
enum TestResult {
    NotRun,                // Automation test was not run
    InProcess,             // Automation test is running now
    Fail,                  // Automation test was run and failed
    Success,               // Automation test was run and succeeded
    NotEnoughParticipants, // Automation test was not run due to number of participan
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Test {
    test_display_name: String,
    full_test_path: String,
    state: TestResult,
    entries: Vec<Entry>,
    warnings: i32,
    errors: i32,
    artifacts: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Device {
    device_name: String,
    instance: String,
    platform: String,
    #[serde(rename = "oSVersion")]
    os_version: String,
    model: String,
    #[serde(rename = "gPU")]
    gpu: String,
    #[serde(rename = "cPUModel")]
    cpu_model: String,
    #[serde(rename = "rAMInGB")]
    ram_in_gb: i32,
    render_mode: String,
    #[serde(rename = "rHI")]
    rhi: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestPass {
    devices: Option<Vec<Device>>,
    report_created_on: String,
    succeeded: i32,
    succeeded_with_warnings: i32,
    failed: i32,
    not_run: i32,
    in_process: Option<i32>,
    total_duration: f32,
    comparison_exported: bool,
    comparison_export_directory: String,
    tests: Vec<Test>,
}

fn utf_from_bytes(buffer: &[u8]) -> String {
    let index_json_string = String::from_utf8_lossy(buffer).into_owned();
    return if index_json_string.starts_with('\u{feff}') {
        String::from_utf8_lossy(&buffer[3..]).into_owned()
    } else {
        index_json_string
    };
}

fn load_file(file_path: &str) -> String {
    let mut file = File::open(file_path).expect("failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("failed to read file");
    utf_from_bytes(&buffer)
}

#[derive(Debug, Deserialize)]
struct TestConfiguration {
    path_to_unrealengine: String,
    path_to_project: String,
    path_to_reports: String,
    run_tests: String,
    test_exit: String,
    ignore_regexes: Vec<String>,
}

fn main() {
    let matches = App::new("runtests")
        .arg(Arg::with_name("tests")
            .help("Sets the tests to run")
            .required(false)
            .multiple(true))
        .arg(Arg::new("config")
            .short('c')
            .long("config")
            .value_name("FILE")
            .default_value("testconfig.toml")
            .help("Sets a custom config file")
            .takes_value(true))
        .get_matches();

    let config_file_path = matches.value_of("config").expect("failed to get config file");

    let config_toml = load_file(config_file_path);
    let config: TestConfiguration = toml::from_str(config_toml.as_str()).expect("failed to parse toml");
    let mut run_tests = config.run_tests.to_owned();

    if let Some(tests) = matches.values_of("tests") {
        if tests.len() > 0 {
            run_tests = tests.into_iter().collect::<Vec<&str>>().join(" ");
        }
    }

    println!("running tests: {}", run_tests);
    let mut run_test_command = Command::new(config.path_to_unrealengine)
        .args([
            config.path_to_project.as_str(),
            format!("-ExecCmds=Automation RunTests {}", run_tests).as_str(),
            "-unattended",
            "-nopause",
            format!("-testexit={}", config.test_exit).as_str(),
            "-game",
            "-log=runtests.log",
            "-NullRHI",
            format!("-ReportOutputPath={}", config.path_to_reports).as_str(),
        ])
        .spawn()
        .expect("failed to start test process");
    println!("process started, waiting for process to finish");

    let test_exit_code = run_test_command.wait().expect("failed to wait for process");
    if !test_exit_code.success() {
        match test_exit_code.code() {
            Some(code) => println!("{}{}", "exited with status code: ".red(), code),
            None => {
                println!("{}", "process terminated by signal".red());
                return;
            }
        }
        return;
    }
    println!("done waiting for process");

    let index_json_string = load_file(format!("{}\\index.json", config.path_to_reports).as_str());
    let index_json = index_json_string.as_str();
    let mut test_pass: TestPass = serde_json::from_str(index_json).expect("invalid json");
    test_pass.tests.sort_by(|a, b| a.full_test_path.cmp(&b.full_test_path));

    let pass_message = "     Success ".bright_green();
    let fail_message = "        Fail ".red();
    let warn_message = "     Warning ".yellow();
    let empty_spacer = "             ";

    let log_info = "        Info ".white();
    let log_warn = "     Warning ".yellow();
    let log_error = "       Error ".red();

    for test in test_pass.tests {
        match test.state {
            TestResult::Success => {
                println!("{}{}", pass_message, test.full_test_path.white());
                for entry in test.entries {

                    if should_ignore_message(entry.event.message.as_str(), &config.ignore_regexes) {
                        continue;
                    }

                    match entry.event.entry_type {
                        EntryType::Warning => {
                            println!("{}{}{}", empty_spacer, log_warn, entry.event.message);
                            println!("{}{}{}:{}", empty_spacer, empty_spacer, entry.filename, entry.line_number);
                        }
                        EntryType::Error => {
                            println!("{}{}{}", empty_spacer, log_error, entry.event.message);
                            println!("{}{}{}:{}", empty_spacer, empty_spacer, entry.filename, entry.line_number);
                        },
                        _ => {}
                    }
                }
            },
            TestResult::Fail => {
                println!("{}{}", fail_message, test.full_test_path.white());
                for entry in test.entries {

                    if should_ignore_message(entry.event.message.as_str(), &config.ignore_regexes) {
                        continue;
                    }

                    match entry.event.entry_type {
                        EntryType::Info => println!("{}{}{}", empty_spacer, log_info, entry.event.message),
                        EntryType::Warning => {
                            println!("{}{}{}", empty_spacer, log_warn, entry.event.message);
                            println!("{}{}{}:{}", empty_spacer, empty_spacer, entry.filename, entry.line_number);
                        }
                        EntryType::Error => {
                            println!("{}{}{}", empty_spacer, log_error, entry.event.message);
                            println!("{}{}{}:{}", empty_spacer, empty_spacer, entry.filename, entry.line_number);
                        }
                    }
                }
            }
            _ => println!("{}{}", warn_message, test.full_test_path.yellow()),
        }
    }

    let succeeded_count = test_pass.succeeded;
    let failed_count = test_pass.failed;
    let other_count = test_pass.not_run + test_pass.succeeded_with_warnings;

    if test_pass.failed > 0 {
        println!("{}", format!("{} passed, {} failed, {} other", succeeded_count, failed_count, other_count).red());
    } else if test_pass.not_run > 0 || test_pass.succeeded_with_warnings > 0 {
        println!("{}", format!("{} passed, {} failed, {} other", succeeded_count, failed_count, other_count).yellow());
    } else {
        println!("{}", format!("{} passed, {} failed, {} other", succeeded_count, failed_count, other_count).bright_green());
    }
    println!("{}s elapsed", test_pass.total_duration);
}

fn should_ignore_message(message: &str, ignore_regexes: &Vec<String>) -> bool {
    for ignore_regex in ignore_regexes.iter() {
        let re = regex::Regex::new(ignore_regex).unwrap();
        if re.is_match(message) {
            return true;
        }
    }
    return false;
}
