//! Where an agent goes when a gate says no (S109 blockers audit, 3 November 2026).
//!
//! A refusal an agent cannot act on is a hard blocker: Reno's Exec needed permission to send
//! email, the owner's card for it existed, and neither of them could reach it. Two lists here keep
//! that class from coming back, and the tests at the bottom fail when either is left incomplete:
//!
//! - [`OWNER_ONLY_DOORS`]: for every owner-only command, the door an agent uses instead. The
//!   refusal in [`crate::wire::authorize`] says it, so the agent learns the way out from the "no".
//! - [`ATTENTION_KINDS`]: every Inbox card kind and who raises it. A card an agent needs must have
//!   an agent command that raises it, and that command must be in the agents' instructions.

/// The way past a refusal, in words the agent acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Door {
    /// The agent asks with this command, which puts a card in the owner's Inbox.
    Ask {
        /// What the agent types, as its instructions spell it.
        cli: &'static str,
        /// The coordination command it sends.
        op: &'static str,
        /// The Inbox card it raises.
        card: &'static str,
    },
    /// The gate has already put this card in the owner's Inbox.
    Raised { card: &'static str },
    /// Only the owner decides this, in this place; the agent tells them so.
    Tell { owner_ui: &'static str },
}

impl Door {
    /// The sentence a refusal ends with.
    pub fn text(&self) -> String {
        match self {
            Self::Ask { cli, .. } => format!(
                "Ask with `{cli}`: it puts the request in the owner's Inbox and you get a message \
                 when they answer. Carry on with other work meanwhile."
            ),
            Self::Raised { .. } => "The owner already has this in their Inbox; you get a message \
                 when they answer. Carry on with other work and don't retry until then."
                .into(),
            Self::Tell { owner_ui } => format!(
                "Only the owner does this, in {owner_ui}. Exec: ask them in your reply and put the \
                 exact ask in `ownerNeed`. Staff: tell Exec with `restless message --to exec`. \
                 Don't retry until they have."
            ),
        }
    }
}

/// A refusal and its way out, as one message.
pub fn refusal_with_door(refusal: impl std::fmt::Display, door: &Door) -> String {
    format!("{refusal}. {}", door.text())
}

const TELL_COMPUTER: Door = Door::Tell {
    owner_ui: "Company › Computer",
};
const TELL_SCHEDULES: Door = Door::Tell {
    owner_ui: "Company › Schedules",
};
const TELL_SPEND: Door = Door::Tell {
    owner_ui: "Company › Spend",
};
const TELL_ACCOUNTING: Door = Door::Tell {
    owner_ui: "Company › Accounting",
};
const TELL_PUBLISHED: Door = Door::Tell {
    owner_ui: "the published service's card in the Inbox",
};
const ASK_SETTING: Door = Door::Ask {
    cli: "restless setting propose",
    op: "setting-propose",
    card: "setting_proposal",
};
const ASK_KEY: Door = Door::Ask {
    cli: "restless apps ask-account",
    op: "account-ask",
    card: "account_request",
};
const ASK_SKILL: Door = Door::Ask {
    cli: "restless skill add",
    op: "skill-candidate-add",
    card: "skill_candidate",
};

/// Every command in [`crate::wire::OWNER_ONLY`], with the door an agent uses instead.
pub const OWNER_ONLY_DOORS: &[(&str, Door)] = &[
    (
        "doctor-collaboration",
        Door::Tell {
            owner_ui: "Company › Doctor",
        },
    ),
    ("schedule-test", TELL_SCHEDULES),
    ("schedule-responsibility-put", TELL_SCHEDULES),
    ("schedule-responsibility-bind", TELL_SCHEDULES),
    ("schedule-responsibility-create", TELL_SCHEDULES),
    ("schedule-wake", TELL_SCHEDULES),
    // A new party is asked about automatically: `restless effect` raises the card.
    (
        "approve",
        Door::Raised {
            card: "approval_required",
        },
    ),
    (
        "decline",
        Door::Raised {
            card: "approval_required",
        },
    ),
    (
        "revoke",
        Door::Tell {
            owner_ui: "Company › Authority",
        },
    ),
    ("up", TELL_COMPUTER),
    ("down", TELL_COMPUTER),
    ("sleep", TELL_COMPUTER),
    ("clear-poison", TELL_COMPUTER),
    ("appliance-drain", TELL_COMPUTER),
    ("appliance-resume", TELL_COMPUTER),
    (
        "platform-recover",
        Door::Tell {
            owner_ui: "Company › Computer, or account › Support for operator recovery",
        },
    ),
    ("spend-correct", TELL_SPEND),
    (
        "company-create",
        Door::Tell {
            owner_ui: "their account page (New company)",
        },
    ),
    ("company-set", ASK_SETTING),
    ("company-unset", ASK_SETTING),
    ("legal-set", ASK_SETTING),
    ("credential-set", ASK_KEY),
    ("credential-promote", ASK_KEY),
    (
        "credential-verify-model",
        Door::Tell {
            owner_ui: "Company › Provider",
        },
    ),
    (
        "finance-envelope-set",
        Door::Ask {
            cli: "restless finance propose-payee",
            op: "finance-payee-propose",
            card: "payee_proposal",
        },
    ),
    ("finance-freeze", TELL_ACCOUNTING),
    ("finance-connect-airwallex", TELL_ACCOUNTING),
    (
        "work-review",
        Door::Raised {
            card: "owner_handoff",
        },
    ),
    (
        "voice-learn",
        Door::Ask {
            cli: "restless identity voice-evidence",
            op: "voice-evidence-add",
            card: "identity_proposal",
        },
    ),
    (
        "publish-authorize",
        Door::Ask {
            cli: "restless publish request",
            op: "publish-request",
            card: "publication_authorization",
        },
    ),
    ("publish-invite", TELL_PUBLISHED),
    ("publish-revoke", TELL_PUBLISHED),
    ("publish-stop", TELL_PUBLISHED),
    ("publish-reconcile", TELL_PUBLISHED),
    ("skill-disposition", ASK_SKILL),
    ("skill-assign", ASK_SKILL),
];

/// The door for one owner-only command.
pub fn owner_only_door(cmd: &str) -> Option<&'static Door> {
    OWNER_ONLY_DOORS
        .iter()
        .find(|(owner_only, _)| *owner_only == cmd)
        .map(|(_, door)| door)
}

/// Who puts a kind of card in the owner's Inbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Raiser {
    /// Restless raises it from what it observes (a spend stop, a machine offline).
    System,
    /// A person raises it (a teammate's message).
    Person,
    /// An agent raises it with this command.
    Agent { cli: &'static str, op: &'static str },
}

/// Every Inbox card kind (`AttentionSource.kind`) and who raises it.
pub const ATTENTION_KINDS: &[(&str, Raiser)] = &[
    (
        "owner_handoff",
        Raiser::Agent {
            cli: "restless work handoff",
            op: "work-handoff",
        },
    ),
    (
        "approval_required",
        Raiser::Agent {
            cli: "restless effect",
            op: "effect",
        },
    ),
    (
        "account_request",
        Raiser::Agent {
            cli: "restless apps ask-account",
            op: "account-ask",
        },
    ),
    (
        "setting_proposal",
        Raiser::Agent {
            cli: "restless setting propose",
            op: "setting-propose",
        },
    ),
    (
        "payee_proposal",
        Raiser::Agent {
            cli: "restless finance propose-payee",
            op: "finance-payee-propose",
        },
    ),
    (
        "publication_authorization",
        Raiser::Agent {
            cli: "restless publish request",
            op: "publish-request",
        },
    ),
    (
        "email_send_request",
        Raiser::Agent {
            cli: "restless email send",
            op: "email-send",
        },
    ),
    // The owner can also propose one from Company settings; Exec asks with this command.
    (
        "email_mandate_proposal",
        Raiser::Agent {
            cli: "restless mandate propose",
            op: "mandate-propose",
        },
    ),
    (
        "direction_request",
        Raiser::Agent {
            cli: "restless request file",
            op: "direction-operation",
        },
    ),
    (
        "charter_proposal",
        Raiser::Agent {
            cli: "restless charter propose",
            op: "direction-operation",
        },
    ),
    (
        "identity_proposal",
        Raiser::Agent {
            cli: "restless identity propose",
            op: "identity-propose",
        },
    ),
    (
        "skill_candidate",
        Raiser::Agent {
            cli: "restless skill add",
            op: "skill-candidate-add",
        },
    ),
    (
        "document_review",
        Raiser::Agent {
            cli: "restless document request-review",
            op: "document-review-request",
        },
    ),
    (
        "document_collaboration",
        Raiser::Agent {
            cli: "restless document request-collaboration",
            op: "document-operation",
        },
    ),
    // Raised by a marker in the agent's reply, which every agent writes.
    ("conversation_owner_need", Raiser::System),
    ("conversation_owner_review", Raiser::System),
    ("teammate_message", Raiser::Person),
    ("connection_sign_in", Raiser::System),
    ("account_notice", Raiser::System),
    ("account_code", Raiser::System),
    ("effect_outcome", Raiser::System),
    ("payment_outcome", Raiser::System),
    ("identity_drift", Raiser::System),
    ("agent_model_unavailable", Raiser::System),
    ("exec_startup_blocked", Raiser::System),
    ("overdue_dispatch_blocked", Raiser::System),
    ("model_cooldown", Raiser::System),
    ("account_headroom", Raiser::System),
    ("spend_warning", Raiser::System),
    ("spend_stopped", Raiser::System),
    ("native_limit_stopped", Raiser::System),
    ("home_helper_offline", Raiser::System),
    ("incoming_email", Raiser::System),
    ("machine_offline", Raiser::System),
    ("work_stalled", Raiser::System),
    ("schedule_run", Raiser::System),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// What agents are told: Exec's context, staff context and every agent prompt.
    fn agent_instructions() -> String {
        [
            include_str!("context.rs"),
            include_str!("staff/context.rs"),
            include_str!("staff/conversation.rs"),
            include_str!("staff/execution.rs"),
            include_str!("../prompts/converse-with-owner.md"),
            include_str!("../prompts/welcome-the-owner.md"),
            crate::owner_brief::DOCTRINE_SKILLS
                .iter()
                .map(|(_, text)| *text)
                .collect::<String>()
                .as_str(),
            include_str!("../../../docs/COMPANY_OPERATING_RULES.md"),
        ]
        .concat()
    }

    /// Non-test code of one source file, without comment lines.
    fn shipped(source: &str) -> String {
        source
            .split("#[cfg(test)]\nmod tests")
            .next()
            .unwrap_or_default()
            .lines()
            .filter(|line| {
                let line = line.trim_start();
                !line.starts_with("//")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn known_kind(kind: &str) -> bool {
        ATTENTION_KINDS.iter().any(|(known, _)| *known == kind)
    }

    #[test]
    fn every_owner_only_command_names_the_agents_door() {
        for cmd in crate::wire::OWNER_ONLY {
            let door = owner_only_door(cmd)
                .unwrap_or_else(|| panic!("owner-only {cmd:?} has no door in OWNER_ONLY_DOORS"));
            match door {
                Door::Ask { cli, op, card } => {
                    assert!(
                        crate::wire::is_command(op),
                        "{cmd}: door command {op:?} does not exist"
                    );
                    assert!(
                        !crate::wire::OWNER_ONLY.contains(op),
                        "{cmd}: door {op:?} is owner-only too, which is a catch-22"
                    );
                    assert!(
                        known_kind(card),
                        "{cmd}: door card {card:?} is not a card kind"
                    );
                    assert!(
                        agent_instructions().contains(cli),
                        "{cmd}: agents are never told about `{cli}`"
                    );
                }
                Door::Raised { card } => {
                    assert!(known_kind(card), "{cmd}: card {card:?} is not a card kind")
                }
                Door::Tell { owner_ui } => assert!(!owner_ui.is_empty()),
            }
            let refusal = crate::wire::authorize(crate::wire::Principal::CompanyExec, cmd)
                .expect_err("owner-only refuses Exec");
            assert!(
                refusal.contains(&door.text()),
                "{cmd}: the refusal does not name its door: {refusal}"
            );
        }
        for (cmd, _) in OWNER_ONLY_DOORS {
            assert!(
                crate::wire::OWNER_ONLY.contains(cmd),
                "{cmd:?} has a door but is not owner-only; remove it"
            );
        }
    }

    #[test]
    fn every_card_an_agent_needs_has_a_command_it_is_told_about() {
        let instructions = agent_instructions();
        for (kind, raiser) in ATTENTION_KINDS {
            let Raiser::Agent { cli, op } = raiser else {
                continue;
            };
            assert!(
                crate::wire::is_command(op),
                "card {kind}: command {op:?} does not exist"
            );
            assert!(
                !crate::wire::OWNER_ONLY.contains(op),
                "card {kind}: only the owner may run {op:?}, so no agent can raise it"
            );
            assert!(
                instructions.contains(cli),
                "card {kind}: agents are never told about `{cli}`"
            );
        }
    }

    #[test]
    fn every_card_kind_says_who_raises_it() {
        let source = shipped(include_str!("attention.rs"));
        let mut kinds = std::collections::BTreeSet::new();
        for piece in source.split("kind: \"").skip(1) {
            let Some((kind, rest)) = piece.split_once('"') else {
                continue;
            };
            if rest.starts_with(".into()") {
                kinds.insert(kind.to_string());
            }
        }
        // Guard the guard: a source move must not leave this checking nothing.
        assert!(kinds.len() > 20, "found only {} card kinds", kinds.len());
        for kind in kinds {
            assert!(
                known_kind(&kind),
                "card kind {kind:?} is not in doors::ATTENTION_KINDS; say who raises it"
            );
        }
        for kind in [
            crate::direction::DIRECTION_REQUEST_ATTENTION_KIND,
            crate::email_approval::REQUEST_KIND,
        ] {
            assert!(known_kind(kind), "card kind {kind:?} is not listed");
        }
    }

    /// The refusals agents read never send them to a command only the owner may run.
    #[test]
    fn agent_facing_text_never_points_at_an_owner_only_command() {
        let owner_only_spellings = [
            "`restless approve",
            "`restless credential set",
            "`restless credential promote",
            "`restless company set",
            "`restless company unset",
            "`restless identity voice-learn",
            "`restless legal set",
            "`restless publish authorize",
            "`restless skill disposition",
            "`restless skill assign",
            "`restless finance envelope",
        ];
        let agent_facing = [
            ("approval.rs", include_str!("approval.rs")),
            ("credential.rs", include_str!("credential.rs")),
            ("effect.rs", include_str!("effect.rs")),
            ("account_broker.rs", include_str!("account_broker.rs")),
            ("tool_gateway.rs", include_str!("tool_gateway.rs")),
            ("account_git.rs", include_str!("account_git.rs")),
            (
                "platform_capabilities.rs",
                include_str!("platform_capabilities.rs"),
            ),
            ("finance.rs", include_str!("finance.rs")),
            ("publication.rs", include_str!("publication.rs")),
            ("home_egress.rs", include_str!("home_egress.rs")),
            ("spend.rs", include_str!("spend.rs")),
            ("setting_proposals.rs", include_str!("setting_proposals.rs")),
            ("coordination.rs", include_str!("coordination.rs")),
            ("context.rs", include_str!("context.rs")),
            ("staff/context.rs", include_str!("staff/context.rs")),
            (
                "staff/conversation.rs",
                include_str!("staff/conversation.rs"),
            ),
        ];
        for (file, source) in agent_facing {
            let source = shipped(source);
            for spelling in owner_only_spellings {
                assert!(
                    !source.contains(spelling),
                    "{file} tells an agent to run {spelling}`, which only the owner may run"
                );
            }
        }
        let mut prompts = include_str!("../prompts/converse-with-owner.md").to_string();
        for (_, text) in crate::owner_brief::DOCTRINE_SKILLS {
            prompts.push_str(text);
        }
        for spelling in owner_only_spellings {
            assert!(
                !prompts.contains(spelling),
                "an agent prompt tells agents to run {spelling}`, which only the owner may run"
            );
        }
    }
}
