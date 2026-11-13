use agentmender_domain::*;
use proptest::prelude::*;

fn metadata(project: Option<&str>) -> SessionMetadata {
    SessionMetadata {
        identity: ArtifactId {
            provider: ProviderKind::Codex,
            state_root: "/fixture".into(),
            class: GraphNodeKind::Session,
            local_id: EntityId::new("session:test").unwrap(),
        },
        native_session_id: "test".into(),
        title: Some("An unrelated title".into()),
        project_label: project.map(str::to_owned),
        created_at: Some(1_792_000_000),
        last_active_at: None,
        state: SessionState::Active,
        producer_version_matches: false,
        producer_version: None,
        parent_native_session_id: None,
    }
}
fn text(value: &str) -> QueryExpr {
    QueryExpr::Text {
        value: value.into(),
        kind: TextMatch::Contains,
        alternatives: vec![],
    }
}
fn project(value: &str) -> QueryExpr {
    QueryExpr::Project {
        values: vec![value.into()],
        source: ProjectSource::ObservedLabel,
    }
}
fn not(arg: QueryExpr) -> QueryExpr {
    QueryExpr::Not { arg: Box::new(arg) }
}
fn facts<'a>(
    metadata: &'a SessionMetadata,
    messages: &'a [QueryText<'a>],
    complete: bool,
) -> QueryFacts<'a> {
    QueryFacts {
        metadata,
        messages,
        content_complete: complete,
        verified_project: None,
        verified_root: None,
    }
}
fn ts(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp()
}

#[test]
fn complete_kleene_truth_tables_and_algebra() {
    use Truth::*;
    let values = [False, Unknown, True];
    let conjunction = [
        [False, False, False],
        [False, Unknown, Unknown],
        [False, Unknown, True],
    ];
    let disjunction = [
        [False, Unknown, True],
        [Unknown, Unknown, True],
        [True, True, True],
    ];
    for (i, a) in values.into_iter().enumerate() {
        for (j, b) in values.into_iter().enumerate() {
            assert_eq!(a.and(b), conjunction[i][j]);
            assert_eq!(a.or(b), disjunction[i][j]);
            assert_eq!(a.and(b).negate(), a.negate().or(b.negate()));
            assert_eq!(a.or(b).negate(), a.negate().and(b.negate()));
            assert_eq!(a.negate().negate(), a);
            for c in values {
                assert_eq!(a.and(b.or(c)), a.and(b).or(a.and(c)));
            }
        }
    }
    assert_eq!(Unknown.negate(), Unknown);
}

#[test]
fn negative_projects_preserve_unknown_and_observations_are_not_verified_ownership() {
    let query = not(project("MISSION1"));
    assert_eq!(
        query.metadata_truth(&metadata(Some("AgentMender"))),
        Truth::True
    );
    assert_eq!(
        query.metadata_truth(&metadata(Some("MISSION1"))),
        Truth::False
    );
    assert_eq!(query.metadata_truth(&metadata(None)), Truth::Unknown);
    let verified = QueryExpr::Project {
        values: vec!["AgentMender".into()],
        source: ProjectSource::VerifiedLabel,
    };
    let m = metadata(Some("AgentMender"));
    assert_eq!(verified.metadata_truth(&m), Truth::Unknown);
    let explicit_unknown = QueryExpr::ProjectUnknown {
        source: ProjectSource::ObservedLabel,
    };
    assert_eq!(
        explicit_unknown.metadata_truth(&metadata(None)),
        Truth::True
    );
    assert_eq!(explicit_unknown.metadata_truth(&m), Truth::False);
}

#[test]
fn title_and_message_predicates_share_bounded_unicode_normalization() {
    let mut m = metadata(None);
    m.title = Some("Carte Pokèmon".into());
    for (value, quality) in [
        ("Carte Pokèmon", TitleMatchQuality::OriginalExact),
        ("CARTE POKEMON", TitleMatchQuality::NormalizedExact),
        ("Carte Poke\u{301}mon", TitleMatchQuality::NormalizedExact),
    ] {
        let title = QueryExpr::Title {
            value: value.into(),
            kind: TitleMatch::Exact,
        };
        assert_eq!(title.metadata_title_match(&m), Some(quality));
        assert_eq!(
            text(value).evaluate(&facts(
                &m,
                &[QueryText {
                    text: "Mi piacciono le carte Pokémon.",
                    timestamp: None,
                }],
                true
            )),
            Truth::True
        );
    }
    let title = QueryExpr::Title {
        value: "Pokémon".into(),
        kind: TitleMatch::Contains,
    };
    assert_eq!(
        title.metadata_title_match(&m),
        Some(TitleMatchQuality::Partial)
    );
    let hybrid = QueryExpr::Or {
        args: vec![title.clone(), text("Pokémon")],
    };
    assert_eq!(hybrid.metadata_truth(&m), Truth::True);
    m.title = None;
    assert_eq!(not(title).metadata_truth(&m), Truth::Unknown);
    assert_eq!(hybrid.metadata_truth(&m), Truth::Unknown);

    for (left, right) in [
        ("año", "ano"),
        ("côté", "cote"),
        ("ＰＯＫＥＭＯＮ", "pokemon"),
    ] {
        assert_ne!(normalize_search_text(left), normalize_search_text(right));
    }
}

#[test]
fn date_ranges_are_half_open_and_calendar_end_dates_inclusive() {
    let range =
        TimeRange::calendar_dates(Some("2026-10-10"), Some("2026-10-20"), "Europe/Rome").unwrap();
    assert_eq!(range.start, Some(ts("2026-10-09T22:00:00Z")));
    assert_eq!(range.end, Some(ts("2026-10-20T22:00:00Z")));
    assert!(range.contains(ts("2026-10-20T21:59:59Z")));
    assert!(!range.contains(ts("2026-10-20T22:00:00Z")));
    // Before October 15 means end exclusive at the start of that local day.
    let before = QueryExpr::Created {
        range: QueryRange::Utc(TimeRange {
            start: None,
            end: Some(ts("2026-10-14T22:00:00Z")),
        }),
    };
    let mut m = metadata(None);
    m.created_at = Some(ts("2026-10-14T21:59:59Z"));
    assert_eq!(before.metadata_truth(&m), Truth::True);
    m.created_at = Some(ts("2026-10-14T22:00:00Z"));
    assert_eq!(before.metadata_truth(&m), Truth::False);
    m.created_at = None;
    assert_eq!(before.metadata_truth(&m), Truth::Unknown);
}

#[test]
fn calendar_days_honor_dst_and_invalid_boundaries_refuse() {
    for (date, seconds) in [("2026-03-29", 23 * 3600), ("2026-10-25", 25 * 3600)] {
        let r = TimeRange::calendar_dates(Some(date), Some(date), "Europe/Rome").unwrap();
        assert_eq!(r.end.unwrap() - r.start.unwrap(), seconds);
    }
    for (start, end, tz) in [
        (Some("2026-02-30"), None, "Europe/Rome"),
        (Some("2026-10-21"), Some("2026-10-20"), "Europe/Rome"),
        (None, None, "Europe/Rome"),
        (Some("2026-01-01"), None, "Invented/Zone"),
        // Samoa skipped this entire local calendar date.
        (Some("2011-12-30"), None, "Pacific/Apia"),
    ] {
        assert!(TimeRange::calendar_dates(start, end, tz).is_err());
    }
}

#[test]
fn combined_filter_is_date_and_negative_project_and_content_disjunction() {
    let q = QueryExpr::And {
        args: vec![
            QueryExpr::Created {
                range: QueryRange::Utc(TimeRange {
                    start: Some(1_790_000_000),
                    end: None,
                }),
            },
            not(project("MISSION1")),
            QueryExpr::Or {
                args: vec![text("PostgreSQL"), text("MongoDB")],
            },
        ],
    };
    let m = metadata(Some("AgentMender"));
    let messages = [QueryText {
        text: "We improved MongoDB indexing.",
        timestamp: None,
    }];
    assert_eq!(q.evaluate(&facts(&m, &messages, false)), Truth::True);
    assert_eq!(
        q.evaluate(&facts(&metadata(Some("MISSION1")), &messages, true)),
        Truth::False
    );
    assert_eq!(
        q.evaluate(&facts(&metadata(None), &messages, true)),
        Truth::Unknown
    );
    assert_eq!(q.metadata_truth(&m), Truth::Unknown);
}

#[test]
fn negative_content_requires_complete_safe_readable_source_and_not_metadata_titles() {
    let q = not(text("Cesium"));
    let mut m = metadata(None);
    m.title = Some("Cesium in title is not conversation content".into());
    let messages = [QueryText {
        text: "Use a database index.",
        timestamp: None,
    }];
    assert_eq!(q.evaluate(&facts(&m, &messages, false)), Truth::Unknown);
    assert_eq!(q.evaluate(&facts(&m, &messages, true)), Truth::True);
    let found = [QueryText {
        text: "Cesium rendering",
        timestamp: None,
    }];
    assert_eq!(q.evaluate(&facts(&m, &found, false)), Truth::False);
}

#[test]
fn incremental_accumulation_matches_cross_page_terms_without_retaining_messages() {
    let q = QueryExpr::And {
        args: vec![text("Earth"), text("Cesium"), not(text("Docker"))],
    };
    let m = metadata(None);
    let mut observed = QueryObservation::default();
    q.observe(
        &mut observed,
        &QueryText {
            text: "Earth rendering",
            timestamp: None,
        },
    );
    assert_eq!(q.evaluate_observed(&m, &observed, false), Truth::Unknown);
    q.observe(
        &mut observed,
        &QueryText {
            text: "Cesium camera updates",
            timestamp: None,
        },
    );
    assert_eq!(q.evaluate_observed(&m, &observed, false), Truth::Unknown);
    assert_eq!(q.evaluate_observed(&m, &observed, true), Truth::True);
    let serialized = serde_json::to_string(&observed).unwrap();
    assert!(!serialized.contains("Earth") && !serialized.contains("Cesium"));
    let forged = QueryObservation {
        matched_atoms: vec![0],
        missing_timestamps: false,
    };
    assert!(!q.observation_valid(&forged));
    assert_eq!(q.evaluate_observed(&m, &forged, true), Truth::Unknown);
}

#[test]
fn missing_message_timestamps_do_not_prove_absence_of_activity() {
    let q = QueryExpr::MessageTime {
        range: QueryRange::Utc(TimeRange {
            start: Some(10),
            end: Some(20),
        }),
    };
    let m = metadata(None);
    for (timestamp, expected) in [
        (None, Truth::Unknown),
        (Some(5), Truth::False),
        (Some(15), Truth::True),
    ] {
        let message = QueryText {
            text: "A message",
            timestamp,
        };
        let mut observed = QueryObservation::default();
        q.observe(&mut observed, &message);
        assert_eq!(q.evaluate(&facts(&m, &[message], true)), expected);
        assert_eq!(q.evaluate_observed(&m, &observed, true), expected);
    }
}

#[test]
fn explicit_query_expansion_and_phrase_exact_matching_are_grounded_lexical_operations() {
    let m = metadata(None);
    let expanded = QueryExpr::Text {
        value: "slow orbital visualization".into(),
        kind: TextMatch::Contains,
        alternatives: vec!["frame drops".into(), "Cesium camera updates".into()],
    };
    assert_eq!(
        expanded.evaluate(&facts(
            &m,
            &[QueryText {
                text: "We optimized Cesium camera updates.",
                timestamp: None
            }],
            true
        )),
        Truth::True
    );
    assert_eq!(
        expanded.text_terms(),
        [
            "cesium camera updates",
            "frame drops",
            "slow orbital visualization"
        ]
    );
    let exact = QueryExpr::Text {
        value: "Earth".into(),
        kind: TextMatch::Exact,
        alternatives: vec![],
    };
    assert_eq!(
        exact.evaluate(&facts(
            &m,
            &[QueryText {
                text: "Earth rendering",
                timestamp: None
            }],
            true
        )),
        Truth::False
    );
    let phrase = QueryExpr::Text {
        value: "Earth rendering".into(),
        kind: TextMatch::Phrase,
        alternatives: vec![],
    };
    assert_eq!(
        phrase.evaluate(&facts(
            &m,
            &[QueryText {
                text: "EARTH\n   rendering is slow",
                timestamp: None
            }],
            true
        )),
        Truth::True
    );
}

#[test]
fn strict_json_validation_rejects_unknown_fields_empty_filters_and_resource_abuse() {
    let q: StructuredQuery = serde_json::from_str(r#"{"filter":{"op":"created","range":{"start_date":"2026-10-10","end_date":"2026-10-20","timezone":"Europe/Rome"}}}"#).unwrap();
    assert!(q.valid());
    for json in [
        r#"{"op":"text","value":"x","shell":"rm"}"#,
        r#"{"op":"created","range":{"start":1,"timezone":"Europe/Rome"}}"#,
        r#"{"op":"provider","values":["unknown"]}"#,
    ] {
        assert!(serde_json::from_str::<QueryExpr>(json).is_err());
    }
    assert!(!QueryExpr::And { args: vec![] }.valid());
    assert!(
        !QueryExpr::Or {
            args: vec![text("x"); 33]
        }
        .valid()
    );
    let mut deep = text("x");
    for _ in 0..13 {
        deep = not(deep);
    }
    assert!(!deep.valid());
    let mut q = q;
    q.sort = SearchSort::Relevance;
    assert!(!q.valid());
    q.mode = SearchMode::Ranked;
    assert!(q.valid());
}

fn expr() -> impl Strategy<Value = QueryExpr> {
    prop_oneof![
        Just(text("alpha")),
        Just(text("beta")),
        Just(project("MISSION1")),
        Just(QueryExpr::MessageTime {
            range: QueryRange::Utc(TimeRange {
                start: Some(10),
                end: Some(20)
            })
        }),
    ]
    .prop_recursive(5, 60, 3, |inner| {
        prop_oneof![
            inner.clone().prop_map(not),
            prop::collection::vec(inner.clone(), 1..4).prop_map(|args| QueryExpr::And { args }),
            prop::collection::vec(inner, 1..4).prop_map(|args| QueryExpr::Or { args }),
        ]
    })
}
proptest! {
    #[test]
    fn streaming_truth_is_independent_of_page_boundaries_and_message_order(
        query in expr(), rows in prop::collection::vec((any::<bool>(), any::<bool>(), prop::option::of(0i64..30)), 0..40),
        complete in any::<bool>(), project_known in any::<bool>(),
    ) {
        let m = metadata(project_known.then_some("MISSION1"));
        let owned:Vec<_> = rows.iter().map(|(a,b,_)| format!("{} {}", if *a {"alpha"} else {"other"}, if *b {"beta"} else {"word"})).collect();
        let messages:Vec<_> = owned.iter().zip(&rows).map(|(text,(_,_,timestamp))| QueryText {text,timestamp:*timestamp}).collect();
        let mut observed = QueryObservation::default();
        for message in &messages { query.observe(&mut observed, message); }
        prop_assert!(query.observation_valid(&observed));
        prop_assert_eq!(query.evaluate(&facts(&m, &messages, complete)), query.evaluate_observed(&m, &observed, complete));
        let mut reverse = QueryObservation::default();
        for message in messages.iter().rev() { query.observe(&mut reverse, message); }
        prop_assert_eq!(observed, reverse);
    }
}
