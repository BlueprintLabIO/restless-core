// GENERATED — do not edit.
//
// Source: crates/restlessd/src/owner.rs and crates/restlessd/src/activity.rs.
// Regenerate: RESTLESS_WRITE_CONVERSATION_BINDINGS=1 cargo test -p restlessd conversation_typescript_bindings_match
//
// Shared owner conversation and live-turn response contract.

export type OwnerAttachment = { uploadId: string, name: string, mediaType: string, sizeBytes: number, path: string, };

export type OwnerIntentKind = "conversation" | "work_feedback" | "direction" | "authority";

export type OwnerIntentReceipt = { kind: OwnerIntentKind, summary: string, };

export type ConversationActorView = { id: string, display: string, kind: string, role: string, };

export type ConversationFocusView = { after_message_id: number, started_at: string | null, };

export type ConversationMessageView = { id: number, from_actor: string, to_actor: string | null, body: string, attachments: Array<OwnerAttachment>, details: string | null, intent: OwnerIntentReceipt | null, context_path: string | null, created_at: string, read_at: string | null, };

export type ConversationView = { actor: ConversationActorView, focus: ConversationFocusView | null, messages: Array<ConversationMessageView>, };

export type ConversationSendResponse = { message_id: number, interrupted: boolean, context_attached: boolean, context_omitted: boolean, focus: ConversationFocusView | null, };

export type ConversationInterruptResponse = { message_id: number, cancelled: boolean, interrupted: boolean, };

export type AgentActivityPhase = "queued" | "thinking" | "acting" | "responding" | "complete" | "failed";

export type AgentActivityItem = { id: string, kind: string, label: string, detail: string, status: string,
/**
 * Unicode-scalar offset into `reply` at which this activity began. The
 * browser uses it to keep visible response/tool chronology intact.
 */
replyOffset: number, };

export type AgentContextUsage = { used: number, size: number, costUsd: number | null, };

export type AgentActivityState = { streamId: string, sequence: number, company: string, actorId: string, triggerMessageId: number | null, workId: string | null, attemptId: string | null, phase: AgentActivityPhase, reply: string,
/**
 * Final generated-output total only when ACP reports it. Context usage
 * below is a separate, live session snapshot.
 */
generatedOutputTokens: number | null, contextUsage: AgentContextUsage | null, activity: Array<AgentActivityItem>, startedAt: string | null, updatedAt: string, completedMessageId: number | null, error: string | null, };
