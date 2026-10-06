//! Restless Core's engine: everything the daemon does beneath the owner
//! API. Company runtimes, OrgIntel, Staff and Exec, authority and effects,
//! harnesses and tools, and the coordination socket the CLI and actors use.
//! The owner API (restless-owner) and the daemon binary sit on top of it.

pub mod acp;
pub mod activity;
pub mod airwallex;
pub mod airwallex_ingress;
pub mod approval;
pub mod attention;
pub mod authority;
pub mod capability;
pub mod capability_sourcing;
pub mod cell;
pub mod cell_wake;
pub mod codex;
pub mod collaboration_doctor;
pub mod company;
pub mod company_bootstrap;
pub mod connections;
pub mod coordination;
pub mod context;
pub mod credential;
pub mod daemon;
pub mod custom_harness;
pub mod document_collaboration_token;
pub mod document_commands;
pub mod documents_service;
pub mod email;
pub mod effect;
pub mod entry;
pub mod exec;
pub mod finance;
pub mod health;
pub mod inbound;
pub mod ingress;
pub mod launch;
pub mod legal;
pub mod local_documents;
pub mod sheet_commands;
pub mod member_access;
pub mod tool_gateway;
pub mod mentions;
pub mod model_catalog;
pub mod model_connections;
pub mod model_gateway;
pub mod native_harness;
pub mod owner_config;
pub mod owner_brief;
pub mod owner_cell_readiness;
pub mod plane;
pub mod publication;
pub mod reconcile;
pub mod release;
pub mod room_commands;
pub mod runtime;
pub mod runtime_bridge;
pub mod runtime_sleep;
pub mod runtime_mode;
pub mod runtime_usage;
pub mod schedule;
pub mod schedule_test;
pub mod schedule_test_proxy;
pub mod skills;
pub mod spend;
pub mod staff;
pub mod telegram;
pub mod telemetry;
pub mod transcript;
pub mod wire;

pub use crate::authority as mandate;

#[allow(unused_imports)]
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use restless_orgintel::OrgIntel;
use serde::{Deserialize, Serialize};
use sqlx::{Connection as _, Executor as _, PgConnection};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[cfg(test)]
use wire::OWNER_ONLY;
use wire::{authorize, Principal, Request, Response};
pub use coordination::*;

/// A daemon task boxed in the crate that defines it. A release build compiles an awaited
/// `async fn`'s state machine again in every crate that awaits it, so restlessd recompiled about
/// 4.9M LLVM lines of upstream code (140s, alone at the end of the build). Handing restlessd
/// these boxed tasks keeps each state machine compiled once, in its own crate.
pub type DaemonTask<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
pub use daemon::*;
