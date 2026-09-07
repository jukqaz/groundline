//! Deterministic, synthetic-only benchmark; never reads a Codex home or network.
use std::fmt::Write;
use std::hint::black_box;
use std::time::Instant;

use groundline_contracts::audit::{AuditWindow, audit_rollouts};
use serde_json::json;
use sha2::{Digest, Sha256};

fn main() {
    let workload = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "unused-payloads".into());
    assert!(
        matches!(
            workload.as_str(),
            "unused-payloads" | "metrics" | "malformed"
        ),
        "workload: unused-payloads, metrics, or malformed"
    );
    let mut contents = String::new();
    writeln!(
        contents,
        "{}",
        json!({"type":"session_meta","payload":{"id":"benchmark","originator":"codex_cli"}})
    )
    .unwrap();
    let world = json!({"timestamp":"2026-09-01T00:00:00Z","type":"world_state","payload":{"unused":(0..256).map(|i| json!({"key":i,"text":"synthetic-record-only".repeat(4)})).collect::<Vec<_>>()}}).to_string();
    for turn in 0..2_000 {
        if workload == "unused-payloads" {
            writeln!(contents, "{world}").unwrap();
        }
        if workload == "malformed" {
            for _ in 0..50 {
                writeln!(contents, "!").unwrap();
            }
        }
        writeln!(contents, "{}", json!({"timestamp":"2026-09-01T00:01:00Z","type":"turn_context","payload":{"model":"gpt-6-astra","effort":"high"}})).unwrap();
        writeln!(contents, "{}", json!({"timestamp":"2026-09-01T00:01:01Z","type":"event_msg","payload":{"type":"user_message","message":"synthetic benchmark task"}})).unwrap();
        writeln!(contents, "{}", json!({"timestamp":"2026-09-01T00:01:02Z","type":"event_msg","payload":{"type":"task_complete","duration_ms":(turn % 100 + 1)*1000}})).unwrap();
        writeln!(contents, "{}", json!({"timestamp":"2026-09-01T00:01:03Z","type":"response_item","payload":{"type":"function_call","name":"exec_command","call_id":format!("call-{turn}"),"arguments":"cargo test"}})).unwrap();
        writeln!(contents, "{}", json!({"timestamp":"2026-09-01T00:01:04Z","type":"response_item","payload":{"type":"function_call_output","call_id":format!("call-{turn}"),"output":{"exit_code":0}}})).unwrap();
        writeln!(contents, "{}", json!({"timestamp":"2026-09-01T00:01:05Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"total_tokens":(turn+1)*100}}}})).unwrap();
    }
    let mut elapsed = Vec::new();
    let mut fingerprint = None;
    for _ in 0..5 {
        let started = Instant::now();
        let result = audit_rollouts(
            &[black_box(&contents)],
            contents.len() as u64,
            20,
            AuditWindow::default(),
        )
        .unwrap();
        elapsed.push(started.elapsed().as_micros());
        let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&result).unwrap()));
        if let Some(expected) = &fingerprint {
            assert_eq!(&digest, expected, "nondeterministic aggregate");
        }
        fingerprint = Some(digest);
    }
    elapsed.sort_unstable();
    println!(
        "{}",
        json!({"kind":"synthetic-audit-benchmark","workload":workload,"profile":if cfg!(debug_assertions){"debug"}else{"release"},"bytes":contents.len(),"records":contents.lines().count(),"iterations":elapsed.len(),"median_us":elapsed[elapsed.len()/2],"min_us":elapsed[0],"max_us":elapsed[elapsed.len()-1],"aggregate_sha256":fingerprint,"reads_private_data":false})
    );
}
