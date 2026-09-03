//! S34 walking slice: five consequence-bearing cases compile only relevant
//! observed conduct, preserve dissent/unknowns/corrections and refuse slogans,
//! self-review and employee-profiling substitutes.
use chrono::Utc;
use restless_orgintel::*;
use std::collections::BTreeSet;

async fn work(org: &OrgIntel, owner: &str, title: &str) -> uuid::Uuid {
    org.add_work(NewWork {
        owner_id: owner,
        title,
        outcome: title,
        goal_id: None,
        priority: 1,
        expected_artifact: "exact case record",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap()
}
fn artifact<'a>(uri: &'a str, digest: &'a str, creator: &'a str) -> NewArtifactRef<'a> {
    NewArtifactRef {
        kind: "culture_case",
        uri,
        note: "exact decision and native communication",
        created_by: creator,
        work_id: None,
        attempt_id: None,
        digest: Some(digest),
        source_commit: None,
        runtime_generation: Some("casebook-v1"),
        label: uri,
    }
}
fn evidence<'a>(
    kind: CultureEvidenceKind,
    case_kind: Option<CultureCase>,
    key: &'a str,
    statement: &'a str,
    conduct: &'a str,
) -> NewCultureEvidence<'a> {
    NewCultureEvidence{kind,case_kind,claim_key:key,statement,author_id:"owner",source:"frozen decision record",authority:"owner",scope:"company",observed_at:Utc::now(),evidence_locator:"casebook@sha256:frozen",polarity:if kind==CultureEvidenceKind::Counterexample{IdentityPolarity::Negative}else{IdentityPolarity::Positive},situation:"A consequential company decision had incomplete evidence and a named accountable actor.",consequence:"The choice affected a customer, delivery quality or the truth of the company record.",actors:"accountable lead, dissenting specialist and owner only at the authority boundary",decision_authority:"the named accountable lead unless owner authority is required",conduct,observed_outcome:"Material alternatives and unknowns remained visible in the final decision account.",confidence:if kind==CultureEvidenceKind::FoundingDecision{CultureConfidence::OwnerFounded}else{CultureConfidence::Corroborated},counterexample:"Fast consensus without preserving a material contradiction produced a plausible but wrong answer.",boundary_conditions:"Do not apply when safety, law, effect authority or owner judgement requires a stricter boundary.",operational_implication:"Surface contradictory evidence once, keep unknowns explicit and preserve the accountable decision.",actor_scope:"company",supersedes_evidence_id:None}
}

#[tokio::test]
async fn culture_is_observed_conduct_not_slogans_or_surveillance() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping culture scenario");
        return;
    };
    let company = format!("companyculture{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    for (id, kind, role, name) in [
        ("owner", "owner", "owner", "Owner"),
        ("customer-support", "staff", "support", "Support lead"),
        ("research-analyst", "staff", "research", "Research analyst"),
        ("software-engineer", "staff", "engineering", "Engineer"),
        ("operations-director", "staff", "exec", "Exec operator"),
        ("independent-reviewer", "staff", "reviewer", "Reviewer"),
    ] {
        org.ensure_actor(id, kind, role, name).await.unwrap();
    }
    let bad = NewCultureEvidence {
        statement: "Ownership",
        conduct: "Ownership",
        ..evidence(
            CultureEvidenceKind::PromotedNorm,
            None,
            "culture.slogan",
            "Ownership",
            "Ownership",
        )
    };
    assert!(org
        .add_culture_evidence(bad)
        .await
        .unwrap_err()
        .to_string()
        .contains("abstract value words"));
    let founding=org.add_culture_evidence(evidence(CultureEvidenceKind::FoundingDecision,None,"culture.method","Preserve a material disagreement until the accountable decision records why one path won.","A specialist sent contradictory evidence directly to the accountable lead; the lead changed the initially favoured decision and recorded both alternatives.")).await.unwrap();
    let dissent=org.add_culture_evidence(evidence(CultureEvidenceKind::ObservedConduct,Some(CultureCase::Disagreement),"culture.dissent","A contradiction may change the answer without changing who decides.","The analyst challenged the favoured answer with source evidence; the lead chose the alternative and attributed the reversal.")).await.unwrap();
    let uncertainty=org.add_culture_evidence(evidence(CultureEvidenceKind::ObservedConduct,Some(CultureCase::UncertainIncident),"culture.unknown","Urgency changes the response speed, not the confidence of an unknown claim.","The incident note separated observed impact, likely cause and unknown scope, then corrected the cause visibly.")).await.unwrap();
    let customer=org.add_culture_evidence(evidence(CultureEvidenceKind::ObservedConduct,Some(CultureCase::CustomerRecovery),"culture.recovery","A customer-visible mistake needs a concrete recovery action and accountable owner.","Support named the missed delivery, known cause, repair deadline and responsible lead before apologetic language.")).await.unwrap();
    let quality=org.add_culture_evidence(evidence(CultureEvidenceKind::Counterexample,Some(CultureCase::QualityTradeoff),"culture.quality","Finishing quickly is not ownership when evidence cannot support the promised outcome.","The lead rejected a polished incomplete result, narrowed scope and returned a verified outcome.")).await.unwrap();
    let hiring=org.add_culture_evidence(evidence(CultureEvidenceKind::PromotedNorm,Some(CultureCase::Hiring),"culture.hiring","Hiring expectations use job scenarios and observable conduct, never personality or culture fit.","The brief asked candidates to handle an uncertain incident, dissent and correction with job-relevant evidence.")).await.unwrap();
    let proposal = org
        .propose_identity_release(
            "customer-support",
            "Release observed operating method.",
            &[founding, dissent, uncertainty, customer, quality, hiring],
        )
        .await
        .unwrap();
    let release = org
        .promote_identity_proposal(
            proposal,
            "owner",
            "authority:culture:release-1",
            "Promoted observed conduct with counterexamples and explicit boundaries.",
            Utc::now(),
        )
        .await
        .unwrap();
    let cases = [
        (
            CultureCase::Disagreement,
            "research-analyst",
            "research",
            "research",
        ),
        (
            CultureCase::UncertainIncident,
            "software-engineer",
            "engineering",
            "engineering",
        ),
        (
            CultureCase::CustomerRecovery,
            "customer-support",
            "support",
            "customer",
        ),
        (
            CultureCase::QualityTradeoff,
            "operations-director",
            "exec",
            "delivery",
        ),
        (CultureCase::Hiring, "operations-director", "exec", "people"),
    ];
    let mut digests = BTreeSet::new();
    let mut records = Vec::new();
    for (index, (case, actor, role, team)) in cases.into_iter().enumerate() {
        let work_id = work(&org, actor, &format!("{case:?} case")).await;
        let contract=org.bind_culture_work_contract(NewCultureWorkContract{work_id,case_kind:case,actor,actor_role:role,team,consequence:"A real decision, customer effect or hiring expectation must remain attributable.",decision_boundary:"Lead may decide within Work; owner only for authority or irreducible judgement.",bound_by:"operations-director"}).await.unwrap();
        assert_eq!(contract.release_id, release);
        let brief = org.compile_culture_posture(work_id, 8192).await.unwrap();
        assert_eq!(
            brief,
            org.compile_culture_posture(work_id, 8192).await.unwrap()
        );
        assert!(brief.body.contains("different defensible decision"));
        assert!(brief.body.contains("No employee score"));
        match case {
            CultureCase::Disagreement => assert!(brief.body.contains("analyst challenged")),
            CultureCase::Hiring => {
                assert!(brief.body.contains("job scenarios"));
                assert!(!brief.body.contains("customer-visible mistake"));
            }
            _ => {}
        }
        digests.insert(brief.digest);
        let uri = format!("/casebook/{index}.html");
        let digest = format!("{:064x}", index + 1);
        let artifact_id = org
            .link_work_artifact(artifact(&uri, &digest, actor))
            .await
            .unwrap();
        let checks = match case {
            CultureCase::Disagreement => {
                serde_json::json!({"dissent_reached_decider":true,"alternatives_preserved":true,"authority_preserved":true})
            }
            CultureCase::UncertainIncident => {
                serde_json::json!({"known_unknown_separated":true,"urgency_did_not_create_certainty":true,"owner_boundary_preserved":true})
            }
            CultureCase::CustomerRecovery => {
                serde_json::json!({"harm_acknowledged":true,"known_facts_named":true,"bounded_action_present":true,"accountable_owner_named":true,"voice_native":true})
            }
            CultureCase::QualityTradeoff => {
                serde_json::json!({"tradeoff_evidence_visible":true,"alternative_preserved":true,"finished_definition_named":true})
            }
            CultureCase::Hiring => {
                serde_json::json!({"job_scenarios_present":true,"observable_conduct_present":true,"no_personality_proxy":true,"no_protected_proxy":true})
            }
        };
        let record=org.record_culture_case(NewCultureCaseRecord{work_id,artifact_ref_id:artifact_id,case_kind:case,decision:if case==CultureCase::Disagreement{"Choose the specialist's alternative after contradictory evidence changed the risk."}else{"Take the bounded evidence-backed action recorded in the artifact."},alternatives:&serde_json::json!(["initially favoured path","evidence-backed alternative"]),unknowns:"Residual uncertainty remains named; nothing absent is treated as zero.",correction_of:None,correction_account:"",customer_action:if case==CultureCase::CustomerRecovery{"Restore the missed service by 16:00; support lead owns confirmation."}else{""},native_checks:&checks,recorded_by:actor}).await.unwrap();
        records.push((case, record, work_id, actor.to_string()));
    }
    assert_eq!(digests.len(), 5);
    let (_, first, _, producer) = &records[0];
    assert!(org
        .record_culture_review(NewCultureReview {
            case_record_id: *first,
            reviewer: producer,
            verdict: CultureReviewVerdict::Accept,
            conduct_findings: "",
            dissent_findings: "",
            uncertainty_findings: "",
            correction_findings: "",
            authority_findings: "",
            customer_or_hiring_findings: "",
            slogan_recitation_detected: false
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("independent reviewer"));
    assert!(org
        .record_culture_review(NewCultureReview {
            case_record_id: *first,
            reviewer: "independent-reviewer",
            verdict: CultureReviewVerdict::Accept,
            conduct_findings: "",
            dissent_findings: "",
            uncertainty_findings: "",
            correction_findings: "",
            authority_findings: "",
            customer_or_hiring_findings: "",
            slogan_recitation_detected: true
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("prose recitation"));
    for (_, record, _, _) in &records {
        org.record_culture_review(NewCultureReview{case_record_id:*record,reviewer:"independent-reviewer",verdict:CultureReviewVerdict::Accept,conduct_findings:"The decision account shows exact conduct and consequence rather than a value label.",dissent_findings:"Material disagreement is preserved where relevant.",uncertainty_findings:"Unknowns remain unknown.",correction_findings:"Original records remain available.",authority_findings:"Role and owner boundaries remain intact.",customer_or_hiring_findings:"Recovery includes action; hiring uses scenarios without proxies.",slogan_recitation_detected:false}).await.unwrap();
    }
    let (_, incident, incident_work, _) = &records[1];
    let correction_artifact = org
        .link_work_artifact(artifact(
            "/casebook/incident-correction.html",
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "software-engineer",
        ))
        .await
        .unwrap();
    let correction=org.record_culture_case(NewCultureCaseRecord{work_id:*incident_work,artifact_ref_id:correction_artifact,case_kind:CultureCase::UncertainIncident,decision:"Correct the suspected cause; customer impact was narrower than first reported.",alternatives:&serde_json::json!(["original suspected cause","new observed cause"]),unknowns:"Long-tail impact remains unknown and is still monitored.",correction_of:Some(*incident),correction_account:"Changed the cause and affected count; preserved the original incident record and current truth.",customer_action:"",native_checks:&serde_json::json!({"known_unknown_separated":true,"urgency_did_not_create_certainty":true,"owner_boundary_preserved":true}),recorded_by:"software-engineer"}).await.unwrap();
    assert_ne!(correction, *incident);
    let snapshot = org.company_identity_snapshot().await.unwrap();
    assert_eq!(snapshot.culture_work_contracts.len(), 5);
    assert_eq!(snapshot.culture_case_records.len(), 6);
    assert_eq!(snapshot.culture_reviews.len(), 5);
    assert!(snapshot
        .culture_reviews
        .iter()
        .all(|review| !review.slogan_recitation_detected));
    org.drop_schema().await.unwrap();
}
