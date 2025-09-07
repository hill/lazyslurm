use anyhow::{Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::collections::HashMap;

use crate::models::{Job, JobState};

pub struct SlurmParser;

impl SlurmParser {
    pub fn parse_squeue_output(output: &str) -> Result<Vec<Job>> {
        let mut jobs = Vec::new();
        
        for line in output.lines() {
            if line.trim().is_empty() || line.starts_with("JOBID") {
                continue;
            }
            
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 4 {
                let job_id = parts[0].trim().to_string();
                let name = parts[1].trim().to_string();
                let user = parts[2].trim().to_string();
                let state = JobState::from(parts[3].trim());
                
                let mut job = Job::new(job_id.clone(), name, user, state);
                
                // Parse array job ID if present (e.g., "23673084_5" -> array_job_id=23673084, task_id=5)
                if job_id.contains('_') {
                    let array_parts: Vec<&str> = job_id.split('_').collect();
                    if array_parts.len() == 2 {
                        job.array_job_id = Some(array_parts[0].to_string());
                        job.array_task_id = array_parts[1].parse().ok();
                    }
                }
                
                // Additional fields if present
                if parts.len() > 4 {
                    job.time_used = Some(parts[4].trim().to_string());
                }
                if parts.len() > 5 {
                    job.node_list = Some(parts[5].trim().to_string());
                }
                if parts.len() > 6 {
                    job.partition = parts[6].trim().to_string();
                }
                
                jobs.push(job);
            }
        }
        
        Ok(jobs)
    }

    pub fn parse_scontrol_output(output: &str) -> Result<HashMap<String, String>> {
        let mut fields = HashMap::new();
        
        // scontrol output format: "Key=Value Key2=Value2 ..."
        // Values can be quoted and contain spaces
        let re = Regex::new(r"(\w+)=([^\s]+(?:\s+[^\s=]+)*)")?;
        
        for line in output.lines() {
            for cap in re.captures_iter(line) {
                let key = cap[1].to_string();
                let value = cap[2].trim_matches('"').to_string();
                fields.insert(key, value);
            }
        }
        
        Ok(fields)
    }

    pub fn enhance_job_with_scontrol_data(job: &mut Job, scontrol_fields: HashMap<String, String>) {
        if let Some(submit_time) = scontrol_fields.get("SubmitTime") {
            job.submit_time = Self::parse_slurm_time(submit_time);
        }
        
        if let Some(start_time) = scontrol_fields.get("StartTime") {
            job.start_time = Self::parse_slurm_time(start_time);
        }
        
        if let Some(end_time) = scontrol_fields.get("EndTime") {
            job.end_time = Self::parse_slurm_time(end_time);
        }
        
        if let Some(working_dir) = scontrol_fields.get("WorkDir") {
            job.working_dir = Some(working_dir.clone());
        }
        
        if let Some(std_out) = scontrol_fields.get("StdOut") {
            job.std_out = Some(std_out.clone());
        }
        
        if let Some(std_err) = scontrol_fields.get("StdErr") {
            job.std_err = Some(std_err.clone());
        }
        
        if let Some(nodes) = scontrol_fields.get("NumNodes") {
            job.nodes = nodes.parse().ok();
        }
        
        if let Some(cpus) = scontrol_fields.get("NumCPUs") {
            job.cpus = cpus.parse().ok();
        }
        
        if let Some(memory) = scontrol_fields.get("MinMemoryNode") {
            job.memory = Some(memory.clone());
        }
        
        if let Some(reason) = scontrol_fields.get("Reason") {
            job.reason = Some(reason.clone());
        }
        
        if let Some(exit_code) = scontrol_fields.get("ExitCode") {
            // Exit code format is usually "0:0" where first is exit code, second is signal
            if let Some(code) = exit_code.split(':').next() {
                job.exit_code = code.parse().ok();
            }
        }
        
        if let Some(time_limit) = scontrol_fields.get("TimeLimit") {
            job.time_limit = Some(time_limit.clone());
        }
    }

    fn parse_slurm_time(time_str: &str) -> Option<DateTime<Utc>> {
        // SLURM time formats: "2024-01-15T10:19:13" or "2024-01-15T10:19:13.123"
        // Sometimes also "Unknown" or "None" for jobs that haven't started
        if time_str == "Unknown" || time_str == "None" || time_str.is_empty() {
            return None;
        }
        
        // Try parsing with seconds
        if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%S") {
            return Some(dt.and_utc());
        }
        
        // Try parsing with microseconds  
        if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%dT%H:%M:%S%.f") {
            return Some(dt.and_utc());
        }
        
        None
    }

    pub fn get_job_log_paths(job: &Job) -> Vec<String> {
        let mut paths = Vec::new();
        
        // Primary: Use the actual StdOut path from scontrol if available
        if let Some(std_out) = &job.std_out {
            paths.push(std_out.clone());
        }
        
        // Secondary: Use StdErr if different
        if let Some(std_err) = &job.std_err && Some(std_err) != job.std_out.as_ref() {
            paths.push(std_err.clone());
        }
        
        // Fallback: Common SLURM default patterns in working directory
        if let Some(work_dir) = &job.working_dir {
            paths.push(format!("{}/slurm-{}.out", work_dir, job.job_id));
            paths.push(format!("{}/slurm-{}.err", work_dir, job.job_id));
        } else {
            // If no working directory known, try current directory
            paths.push(format!("slurm-{}.out", job.job_id));
            paths.push(format!("slurm-{}.err", job.job_id));
        }
        
        // Additional fallback: Check /tmp for logs (common in dev environments)
        paths.push(format!("/tmp/slurm-{}.out", job.job_id));
        paths.push(format!("/tmp/slurm-{}.err", job.job_id));
        
        paths
    }
}
