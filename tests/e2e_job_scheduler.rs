//! E2E integration tests for the `fabio job-scheduler` command group.
//!
//! Tests job instance listing and schedule operations against live Fabric API.

mod common;

use common::{TestConfig, extract_count, extract_data, fabio, parse_json};

#[test]
#[ignore = "requires live Fabric tenant"]
fn job_scheduler_list_instances() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "job-scheduler",
            "list-instances",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    // Should return a list (possibly empty for items with no job history)
    assert!(json.get("data").is_some() || json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn job_scheduler_list_instances_with_limit() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "job-scheduler",
            "list-instances",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
            "--limit",
            "1",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let count = extract_count(&json);
    assert!(count <= 1);
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn job_scheduler_run_on_demand_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "job-scheduler",
            "run-on-demand",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
            "--job-type",
            "RunNotebook",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn job_scheduler_cancel_instance_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "job-scheduler",
            "cancel-instance",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
            "--job-instance-id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn job_scheduler_list_schedules() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "job-scheduler",
            "list-schedules",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
            "--job-type",
            "RunNotebook",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    // Should return a list (possibly empty)
    assert!(json.get("data").is_some() || json.get("count").is_some());
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn job_scheduler_create_schedule_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "job-scheduler",
            "create-schedule",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
            "--job-type",
            "RunNotebook",
            "--config",
            r#"{"type":"Cron","expression":"0 0 * * *","timezone":"UTC"}"#,
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}

#[test]
#[ignore = "requires live Fabric tenant"]
fn job_scheduler_delete_schedule_dry_run() {
    let cfg = TestConfig::from_env();

    let output = fabio()
        .args([
            "job-scheduler",
            "delete-schedule",
            "--workspace",
            &cfg.source_workspace,
            "--id",
            &cfg.notebook_id,
            "--job-type",
            "RunNotebook",
            "--schedule-id",
            "00000000-0000-0000-0000-000000000000",
            "--dry-run",
        ])
        .assert()
        .success();

    let json = parse_json(&output);
    let data = extract_data(&json);
    assert_eq!(data["status"], "dry_run");
}
