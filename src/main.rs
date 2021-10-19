use serde::{Deserialize};
use std::fs::File;
use std::io::prelude::*;
use colored::*;

#[derive(Debug, Deserialize)]
enum EntryType {
    Info,
	Warning,
	Error,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    r#type: EntryType,
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
    NotRun,					// Automation test was not run
	InProcess,				// Automation test is running now
	Fail,					// Automation test was run and failed
	Success,				// Automation test was run and succeeded
	NotEnoughParticipants,	// Automation test was not run due to number of participan
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
    #[serde(rename="oSVersion")]
    os_version: String,
    model: String,
    #[serde(rename="gPU")]
    gpu: String,
    #[serde(rename="cPUModel")]
    cpu_model: String,
    #[serde(rename="rAMInGB")]
    ram_in_gb: i32,
    render_mode: String,
    #[serde(rename="rHI")]
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
    let index_json_string = String::from_utf8_lossy(&buffer).into_owned();
    return if index_json_string.starts_with("\u{feff}") {
        String::from_utf8_lossy(&buffer[3..]).into_owned()
    } else {
        index_json_string
    };
}

fn main() {
    let mut file = File::open("index2.json").expect("failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("failed to read file");
    let index_json_string = utf_from_bytes(&buffer);
    let index_json = index_json_string.as_str();
    let mut test_pass : TestPass = serde_json::from_str(index_json).expect("invalid json");
    test_pass.tests.sort_by(|a, b| a.full_test_path.cmp(&b.full_test_path));

    let pass_message = "     Success ".bright_green();
    let fail_message = "        Fail ".red();
    let warn_message = "     Warning ".yellow();
    let empty_spacer = "             ";

    let log_info = "        Info ".white();
    let log_warn = "     Warning ".yellow();
    let log_error ="       Error ".red();

    for test in test_pass.tests {
        match test.state {
            
            TestResult::Success => println!("{}{}", pass_message, test.full_test_path.white()),
            TestResult::Fail => 
            {
                println!("{}{}", fail_message, test.full_test_path.white());
                for entry in test.entries {
                    match entry.event.r#type {
                        EntryType::Info => println!("{}{}{}", empty_spacer, log_info, entry.event.message),
                        EntryType::Warning => {
                            println!("{}{}{}", empty_spacer, log_warn, entry.event.message);
                            println!("{}{}{}:{}", empty_spacer, empty_spacer, entry.filename, entry.line_number);
                        },
                        EntryType::Error => {
                            println!("{}{}{}", empty_spacer, log_error, entry.event.message);
                            println!("{}{}{}:{}", empty_spacer, empty_spacer, entry.filename, entry.line_number);
                        }
                    }
                }
            },
            _ => println!("{}{}", warn_message, test.full_test_path.yellow()),
        }
        
    }

    if test_pass.failed > 0 {
        println!("{}", format!("{} passed, {} failed, {} other", test_pass.succeeded, test_pass.failed, test_pass.not_run + test_pass.succeeded_with_warnings).red());
    }
    else if test_pass.not_run > 0 || test_pass.succeeded_with_warnings > 0 {
        println!("{}", format!("{} passed, {} failed, {} other", test_pass.succeeded, test_pass.failed, test_pass.not_run + test_pass.succeeded_with_warnings).yellow());
    }
    else
    {
        println!("{}", format!("{} passed, {} failed, {} other", test_pass.succeeded, test_pass.failed, test_pass.not_run + test_pass.succeeded_with_warnings).bright_green());
    }
    println!("{}s elapsed", test_pass.total_duration);
}
