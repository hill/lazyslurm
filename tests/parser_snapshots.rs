//! Snapshot tests for the SLURM output parsers.
//!
//! These tests pin parser behavior against captured fixture outputs. To accept
//! intentional changes, run `cargo insta review` (or `cargo insta accept`).

use std::path::PathBuf;

use lazyslurm::slurm::{SlurmExecutor, SlurmFixture, SlurmParser};

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[tokio::test]
async fn parse_squeue_basic() {
    let exec = SlurmFixture::new(fixture_dir("basic"));
    let raw = exec.squeue(None, None).await.unwrap();
    let jobs = SlurmParser::parse_squeue_output(&raw).unwrap();
    insta::assert_yaml_snapshot!(jobs);
}

#[tokio::test]
async fn parse_squeue_empty() {
    let exec = SlurmFixture::new(fixture_dir("empty"));
    let raw = exec.squeue(None, None).await.unwrap();
    let jobs = SlurmParser::parse_squeue_output(&raw).unwrap();
    insta::assert_yaml_snapshot!(jobs);
}

#[tokio::test]
async fn parse_squeue_array_jobs() {
    let exec = SlurmFixture::new(fixture_dir("array_jobs"));
    let raw = exec.squeue(None, None).await.unwrap();
    let jobs = SlurmParser::parse_squeue_output(&raw).unwrap();
    insta::assert_yaml_snapshot!(jobs);
}

#[tokio::test]
async fn parse_scontrol_running_job() {
    let exec = SlurmFixture::new(fixture_dir("basic"));
    let raw = exec.scontrol_show_job("12345").await.unwrap();
    let mut fields: Vec<(String, String)> = SlurmParser::parse_scontrol_output(&raw)
        .unwrap()
        .into_iter()
        .collect();
    // HashMap order is non-deterministic; sort for stable snapshots.
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    insta::assert_yaml_snapshot!(fields);
}

#[tokio::test]
async fn parse_scontrol_pending_job() {
    let exec = SlurmFixture::new(fixture_dir("basic"));
    let raw = exec.scontrol_show_job("12346").await.unwrap();
    let mut fields: Vec<(String, String)> = SlurmParser::parse_scontrol_output(&raw)
        .unwrap()
        .into_iter()
        .collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    insta::assert_yaml_snapshot!(fields);
}

#[tokio::test]
async fn enhance_job_with_scontrol() {
    let exec = SlurmFixture::new(fixture_dir("basic"));
    let raw_squeue = exec.squeue(None, None).await.unwrap();
    let mut jobs = SlurmParser::parse_squeue_output(&raw_squeue).unwrap();

    for job in jobs.iter_mut() {
        if let Ok(raw_scontrol) = exec.scontrol_show_job(&job.job_id).await
            && let Ok(fields) = SlurmParser::parse_scontrol_output(&raw_scontrol)
        {
            SlurmParser::enhance_job_with_scontrol_data(job, fields);
        }
    }

    insta::assert_yaml_snapshot!(jobs);
}

#[tokio::test]
async fn fixture_scancel_records_calls() {
    let exec = SlurmFixture::new(fixture_dir("basic"));
    exec.scancel("12345").await.unwrap();
    exec.scancel("12347").await.unwrap();
    let cancelled = exec.cancelled.lock().unwrap().clone();
    assert_eq!(cancelled, vec!["12345", "12347"]);
}

// The "cluster" fixture stands in for a realistic multi-user cluster: several
// users and accounts, multiple partitions, QOS tiers, fake GPUs, a job array,
// a dependency, and a spread of finished-job states for the History tab.

#[tokio::test]
async fn parse_squeue_cluster() {
    let exec = SlurmFixture::new(fixture_dir("cluster"));
    let raw = exec.squeue(None, None).await.unwrap();
    let jobs = SlurmParser::parse_squeue_output(&raw).unwrap();
    insta::assert_yaml_snapshot!(jobs);
}

#[tokio::test]
async fn parse_sinfo_nodes_cluster() {
    let exec = SlurmFixture::new(fixture_dir("cluster"));
    let raw = exec.sinfo_nodes().await.unwrap();
    let nodes = SlurmParser::parse_sinfo_nodes(&raw);
    insta::assert_yaml_snapshot!(nodes);
}

#[tokio::test]
async fn parse_sinfo_partitions_cluster() {
    let exec = SlurmFixture::new(fixture_dir("cluster"));
    let raw = exec.sinfo_partitions().await.unwrap();
    let partitions = SlurmParser::parse_sinfo_partitions(&raw);
    insta::assert_yaml_snapshot!(partitions);
}

#[tokio::test]
async fn parse_sacct_cluster() {
    let exec = SlurmFixture::new(fixture_dir("cluster"));
    let raw = exec.sacct(None).await.unwrap();
    let entries = SlurmParser::parse_sacct(&raw);
    insta::assert_yaml_snapshot!(entries);
}

#[tokio::test]
async fn parse_sacct_detail_cluster() {
    let exec = SlurmFixture::new(fixture_dir("cluster"));
    let raw = exec.sacct_job("48100").await.unwrap();
    let detail = SlurmParser::parse_sacct_detail(&raw, "48100");
    insta::assert_yaml_snapshot!(detail);
}

#[tokio::test]
async fn parse_scontrol_cluster_gpu_job() {
    let exec = SlurmFixture::new(fixture_dir("cluster"));
    let raw = exec.scontrol_show_job("48201").await.unwrap();
    let mut fields: Vec<(String, String)> = SlurmParser::parse_scontrol_output(&raw)
        .unwrap()
        .into_iter()
        .collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    insta::assert_yaml_snapshot!(fields);
}
