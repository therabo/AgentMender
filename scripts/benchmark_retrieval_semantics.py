#!/usr/bin/env python3
"""Offline retrieval experiment over hand-labelled synthetic coding sessions."""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import re
import sqlite3
import statistics
import time

# Each topic has one supported original and two hard distractors. Query terms
# and alternative terminology are supplied independently of the ranker.
TOPICS = [
    ("earth", "Dove abbiamo limitato quanto ci si può avvicinare alla Terra?",
     ["limitato", "avvicinare", "Terra"], ["WGS84", "camera bounds"],
     "Implemented WGS84 camera bounds for orbital rendering.",
     "Viewport regression checks verified min and max altitude.",
     "Considered clipping ground altitude but deferred the implementation.",
     "A retrospective points to another session's WGS84 camera bounds."),
    ("frames", "Where did we fix the sluggish spacecraft display?",
     ["sluggish", "spacecraft", "display"], ["frame pacing", "Cesium"],
     "Batched Cesium scene updates and fixed frame pacing.",
     "Confirmed smooth rendering after the camera update patch.",
     "Observed a sluggish display but made no change.",
     "The retrospective lists Cesium frame pacing changes done elsewhere."),
    ("permission", "Dove abbiamo risolto l'accesso negato in lettura?",
     ["accesso negato", "lettura"], ["EPERM", "O_SEARCH"],
     "Implemented the O_SEARCH directory-descriptor fix for EPERM.",
     "The protected read succeeds and write denial is preserved.",
     "Investigated read access restrictions and documented possible fixes.",
     "A status recap links to the separate EPERM O_SEARCH implementation."),
    ("telemetry", "Where did the station data make it all the way to the screen?",
     ["station data", "screen"], ["ISSLIVE", "telemetry"],
     "Implemented ISSLIVE telemetry from the provider to REST and UI.",
     "A smoke test traced the live path through PostgreSQL to the client.",
     "Discussed station data authentication and possible screens.",
     "A follow-up mentions ISSLIVE telemetry implemented in a prior chat."),
    ("bootstrap", "Dove abbiamo dimostrato l'avvio su un database vergine?",
     ["avvio", "database vergine"], ["Flyway", "empty database"],
     "Verified PostgreSQL bootstrap on two empty databases with Flyway.",
     "Both cold starts executed initial migrations successfully.",
     "Proposed a database reset to inspect future migrations.",
     "A summary quotes the Flyway empty database result from another chat."),
    ("rollback", "Where did we undo the atmospheric shortcut after testing?",
     ["undo", "atmospheric shortcut"], ["cloud fidelity", "reverted"],
     "Removed the temporary low-detail setting after a fidelity test failed.",
     "We reverted the atmosphere workaround and restored cloud fidelity.",
     "Suggested disabling clouds as a temporary workaround.",
     "A retrospective mentions the cloud fidelity workaround reverted elsewhere."),
    ("retry", "Dove abbiamo impedito un tentativo automatico dopo un esito incerto?",
     ["tentativo automatico", "esito incerto"], ["OutcomeUnknown", "journal"],
     "Implemented journal reconciliation for OutcomeUnknown.",
     "The engine now refuses automatic dispatch retries on interrupted effects.",
     "A design note proposed a manual retry for known failures.",
     "The report cites OutcomeUnknown journal changes from the implementation chat."),
    ("compress", "Where did large archived chats become readable again?",
     ["archived chats", "readable"], ["Zstandard", "rollout"],
     "Added Zstandard rollout traversal with a resident decoder.",
     "A generated compressed history reached physical EOF under bounded work.",
     "Discussed exporting archived chats into a separate store.",
     "The release notes describe Zstandard rollout work performed elsewhere."),
    ("filters", "Dove abbiamo corretto la data durante il cambio di ora?",
     ["cambio di ora", "data"], ["Europe/Rome", "UTC interval"],
     "Verified the Europe/Rome fallback day is a 25-hour UTC interval.",
     "The date filter returns the exact same rows as half-open UTC bounds.",
     "Planned to add a timezone field to the date picker.",
     "An agenda mentions the Europe/Rome test from the prior chat."),
    ("wal", "Where did we establish who could write while the app stayed open?",
     ["write", "app stayed open"], ["SQLite WAL", "writer fence"],
     "Tested the SQLite WAL writer fence on a disposable provider.",
     "A second writer was refused throughout the transaction boundary.",
     "Considered asking users to close the app and retry writes.",
     "This summary links to the separate SQLite WAL writer fence experiment."),
    ("worker", "Dove abbiamo scoperto che tutti i nodi avevano perso il collegamento?",
     ["nodi", "collegamento"], ["gRPC", "worker"],
     "Confirmed gRPC connectivity loss across all worker processes.",
     "The corrected incident analysis counts 45 affected workers.",
     "Discussed one worker going offline during deployment.",
     "The incident recap quotes the gRPC worker finding from a different chat."),
    ("leveldb", "Where did we separate disposable cache keys from saved session data?",
     ["disposable cache", "saved session data"], ["LevelDB", "classifier"],
     "Implemented a LevelDB classifier for retained session data.",
     "The fixture retained ambiguous records and classified known cache keys.",
     "An earlier proposal considered clearing browser caches.",
     "A later review cites the LevelDB classifier from the implementation chat."),
]


def samples():
    needs = []
    passages = []
    for topic, question, literal, aliases, original, outcome, discussion, retrospective in TOPICS:
        needs.append((topic, question, literal, aliases))
        passages.extend([
            (topic, original, outcome), (topic + "-discussion", discussion, ""),
            (topic + "-retrospective", retrospective, ""),
        ])
    return needs, passages


def fts_index(passages, contextual):
    db = sqlite3.connect(":memory:")
    db.execute("CREATE VIRTUAL TABLE passages USING fts5(session UNINDEXED, body, tokenize='unicode61')")
    db.executemany("INSERT INTO passages VALUES(?,?)",
                   [(sid, f"{body} {adjacent}" if contextual else body) for sid, body, adjacent in passages])
    return db


def lexical(db, terms):
    words = [word for term in terms for word in re.findall(r"\w+", term)]
    query = " OR ".join('"' + word + '"' for word in words)
    return [row[0] for row in db.execute(
        "SELECT session FROM passages WHERE passages MATCH ? ORDER BY bm25(passages),session", (query,))]


def fusion(*lists):
    scores = defaultdict(float)
    for sequence in lists:
        for rank, sid in enumerate(sequence, 1):
            scores[sid] += 1 / (60 + rank)
    return sorted(scores, key=lambda sid: (-scores[sid], sid))


def encode(session, tokenizer, texts):
    import numpy as np

    items = tokenizer.encode_batch(texts)
    tensors = {name: np.asarray([getattr(item, attribute) for item in items], dtype=np.int64)
               for name, attribute in [("input_ids", "ids"), ("attention_mask", "attention_mask"),
                                       ("token_type_ids", "type_ids")]}
    inputs = {item.name: tensors[item.name] for item in session.get_inputs()}
    hidden = session.run(None, inputs)[0]
    mask = tensors["attention_mask"][..., None]
    pooled = (hidden * mask).sum(axis=1) / mask.sum(axis=1)
    return pooled / np.linalg.norm(pooled, axis=1, keepdims=True)


def evaluate(model, repeats):
    import numpy as np
    import onnxruntime as ort
    from tokenizers import Tokenizer

    needs, passages = samples()
    db, contextual = fts_index(passages, False), fts_index(passages, True)
    started = time.perf_counter_ns()
    options = ort.SessionOptions()
    options.intra_op_num_threads = 2
    options.inter_op_num_threads = 1
    session = ort.InferenceSession(str(model / "model.onnx"), options, providers=["CPUExecutionProvider"])
    tokenizer = Tokenizer.from_file(str(model / "tokenizer.json"))
    tokenizer.enable_padding()
    tokenizer.enable_truncation(max_length=128)
    vectors = encode(session, tokenizer, ["passage: " + body + " " + adjacent
                                         for _, body, adjacent in passages])
    build_ms = (time.perf_counter_ns() - started) / 1e6
    metrics = defaultdict(list)
    latencies = defaultdict(list)
    per_query = {}
    for expected, question, literal, aliases in needs:
        for _ in range(repeats):
            start = time.perf_counter_ns()
            baseline = lexical(db, literal)
            latencies["lexical"].append((time.perf_counter_ns() - start) / 1e6)
            start = time.perf_counter_ns()
            rewritten = lexical(db, literal + aliases)
            latencies["aliases"].append((time.perf_counter_ns() - start) / 1e6)
            start = time.perf_counter_ns()
            adjacent = lexical(contextual, literal + aliases)
            latencies["contextual"].append((time.perf_counter_ns() - start) / 1e6)
            start = time.perf_counter_ns()
            embedding = encode(session, tokenizer, ["query: " + question])[0]
            similarity = vectors @ embedding
            dense = [passages[i][0] for i in np.argsort(-similarity, kind="stable")]
            dense_ms = (time.perf_counter_ns() - start) / 1e6
            latencies["dense"].append(dense_ms)
            start = time.perf_counter_ns()
            hybrid = fusion(adjacent[:10], dense[:10])
            latencies["hybrid"].append(
                dense_ms + latencies["contextual"][-1] + (time.perf_counter_ns() - start) / 1e6)
        ranks = {}
        for name, results in [("lexical", baseline), ("aliases", rewritten), ("contextual", adjacent),
                              ("dense", dense), ("hybrid", hybrid)]:
            rank = results.index(expected) + 1 if expected in results else None
            ranks[name] = rank
            metrics[name].append((rank == 1, rank is not None and rank <= 5, 1 / rank if rank else 0))
        per_query[expected] = ranks
    return {"topics": len(needs), "passages": len(passages), "repeats": repeats, "build_ms": build_ms,
            "model_bytes": (model / "model.onnx").stat().st_size, "per_query": per_query,
            "results": {name: {"precision_at_1": statistics.mean(row[0] for row in rows),
                               "recall_at_5": statistics.mean(row[1] for row in rows),
                               "mrr": statistics.mean(row[2] for row in rows),
                               "query_p50_ms": statistics.median(latencies[name])}
                        for name, rows in metrics.items()}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("model_directory", type=Path, help="local int8 E5 ONNX and tokenizer.json")
    parser.add_argument("--repeats", type=int, default=5)
    args = parser.parse_args()
    if args.repeats < 1:
        parser.error("repeats must be positive")
    print(json.dumps(evaluate(args.model_directory, args.repeats), indent=2))


if __name__ == "__main__":
    main()
