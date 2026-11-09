//! Reproducible sanitized retrieval/storage microbenchmark; no provider reads.
use agentmender_persistence::{IndexDocument, SearchTextIndex, open_search_index};
use std::{collections::BTreeSet, time::Instant};
fn metrics(results: &[usize], truth: &BTreeSet<usize>) -> serde_json::Value {
    let hit = |n: usize| {
        results
            .iter()
            .take(n)
            .filter(|id| truth.contains(id))
            .count()
    };
    let rr = results
        .iter()
        .position(|id| truth.contains(id))
        .map_or(0.0, |p| 1.0 / (p + 1) as f64);
    serde_json::json!({"recall_at_10":hit(10) as f64/truth.len() as f64,"precision_at_5":hit(5) as f64/5.0,"mrr":rr,"false_negative_rate":1.0-hit(results.len()) as f64/truth.len() as f64})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let protected = temp.path().join("disposable-provider");
    std::fs::create_dir(&protected)?;
    let path = temp.path().join("private-index");
    let mut index = open_search_index(&path, &[protected])?;
    let started = Instant::now();
    let mut input_bytes = 0;
    // Ground truth: ten orbital sessions. Message20 holds the problem, 35 rejects
    // a workaround, 38 confirms the fix. All other sessions concern packaging.
    let mut truth = BTreeSet::new();
    for session in 0..1000 {
        let relevant = session % 100 == 0;
        if relevant {
            truth.insert(session);
        }
        let messages:Vec<_>=(0..40).map(|message|match (relevant,message) {
            (true,20)=>"Cesium camera updates cause frame drops and rendering bottlenecks.".to_string(),
            (true,35)=>"We rejected disabling atmospheric clouds because fidelity is required.".to_string(),
            (true,38)=>"Batching camera updates fixed frame pacing; the user confirmed smooth orbital visualization.".to_string(),
            _=>format!("Packaging maintenance discussion {}. {}",message,"Routine dependency review and release notes. ".repeat(8)),
        }).collect();
        input_bytes += messages.iter().map(String::len).sum::<usize>();
        let refs: Vec<_> = (0..40).map(|m| format!("fixture:{session}:{m}")).collect();
        let docs: Vec<_> = messages
            .iter()
            .zip(&refs)
            .enumerate()
            .map(|(ordinal, (text, reference))| IndexDocument {
                ordinal: ordinal as u64,
                chunk: 0,
                text,
                reference,
            })
            .collect();
        index.append_page(
            &format!("session:{session:04}"),
            "fixture-rev1",
            0,
            40,
            true,
            &docs,
        )?;
    }
    let build_ms = started.elapsed().as_secs_f64() * 1000.0;
    let query_started = Instant::now();
    let mut ranked = Vec::new();
    for found in index.search("Cesium rendering", 1000)? {
        let id = found
            .source
            .trim_start_matches("session:")
            .parse::<usize>()?;
        if !ranked.contains(&id) {
            ranked.push(id);
        }
    }
    let lexical_us = query_started.elapsed().as_secs_f64() * 1e6;
    // Semantic wording intentionally shares no topic tokens with the problem.
    let semantic_raw = index.search("sluggish spacecraft display", 1000)?;
    let expanded = index.search("frame drops rendering bottlenecks", 1000)?;
    let unique = |rows: Vec<agentmender_persistence::IndexMatch>| {
        let mut ids = Vec::new();
        for row in rows {
            let id = row
                .source
                .trim_start_matches("session:")
                .parse::<usize>()
                .unwrap();
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    };
    let incremental = Instant::now();
    index.append_page(
        "session:0000",
        "fixture-rev1",
        40,
        41,
        true,
        &[IndexDocument {
            ordinal: 40,
            chunk: 0,
            text: "Confirmed stable graphics workload after camera update batching.",
            reference: "fixture:0:40",
        }],
    )?;
    let update_us = incremental.elapsed().as_secs_f64() * 1e6;
    index.integrity_check()?;
    let status = index.status()?;
    let mut ephemeral = SearchTextIndex::memory()?;
    ephemeral.add(1, "Cesium rendering")?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "dataset":{"sessions":1000,"messages":40000,"input_text_bytes":input_bytes,"relevant_sessions":truth,"privacy":"generated synthetic text only","position_note":"middle and later occurrences; provider scanner separately tests full byte budgets"},
            "runtime":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"profile":"debug/no-debuginfo","sqlite":status.sqlite_version},
            "measurements":{"initial_build_ms":build_ms,"incremental_append_us":update_us,"lexical_query_us":lexical_us,"index_disk_bytes":status.database_bytes},
            "quality":{"fts5_lexical":metrics(&ranked,&truth),"literal_semantic_wording":metrics(&unique(semantic_raw),&truth),"explicit_lexical_expansion":metrics(&unique(expanded),&truth),"luna_rerank":"separate bounded synthetic subagent acceptance; no statistical quality claim","p0_s1":"provider fixture tests compare old head/tail omission; no fabricated benchmark number"},
            "limits":["microbenchmark; no production latency guarantee","not a multilingual or general semantic recall evaluation","BM25 candidate ranking; exact filters and coverage separately tested","includes opted-in safe plaintext plus contentless FTS postings"]
        }))?
    );
    Ok(())
}
