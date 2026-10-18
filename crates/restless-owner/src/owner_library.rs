//! Everyday Library work the owner does directly (S64): adding files to the company without a
//! message, and archiving or restoring a file Work produced. Uploaded files use the chat
//! attachment substrate exactly: the same limits, staging, integrity record and purge path. The
//! upload shares the owner API's 32 MB body ceiling with a chat message carrying six files.
use super::*;

#[derive(Debug, Serialize)]
struct UploadedAttachment {
    id: Uuid,
    name: String,
    media_type: String,
    size_bytes: usize,
}

/// A full quota is the owner's to resolve, so it says what to do; a busy staging area clears on
/// its own and stays retryable. Any path that registers owner attachments can answer with this.
pub(super) fn registration_refusal(error: restless_orgintel::OrgIntelError) -> Response<Body> {
    match error {
        restless_orgintel::OrgIntelError::InvalidRoom(message)
            if message.contains("retained attachment storage is full") =>
        {
            api_error(
                StatusCode::CONFLICT,
                "attachment_quota",
                format!(
                    "The company's file storage is full ({MAX_RETAINED_ATTACHMENT_FILES} files or {} MB). Delete files you no longer need from Library → From you, then try again.",
                    MAX_RETAINED_ATTACHMENT_BYTES / (1024 * 1024)
                ),
            )
        }
        restless_orgintel::OrgIntelError::InvalidRoom(message)
            if message.contains("retained attachment quota is full") =>
        {
            api_error(
                StatusCode::CONFLICT,
                "attachment_quota",
                format!(
                    "Your kept files are full ({MAX_PRINCIPAL_RETAINED_ATTACHMENT_FILES} files or {} MB). Delete files you no longer need from Library → From you, then try again.",
                    MAX_PRINCIPAL_RETAINED_ATTACHMENT_BYTES / (1024 * 1024)
                ),
            )
        }
        restless_orgintel::OrgIntelError::InvalidRoom(message)
            if message.contains("attachment staging is full") =>
        {
            api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "attachment_staging",
                "Other files are still being stored. Try again in a moment.",
            )
        }
        error => {
            tracing::warn!(%error, "owner attachment registration failed");
            files_not_stored()
        }
    }
}

/// The owner reads what to do; the internal cause goes to the log.
fn files_not_stored() -> Response<Body> {
    api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "attachment_staging",
        "The files could not be stored on the company computer. Try again in a moment.",
    )
}

/// POST /companies/{company}/attachments: one or more parts named `file`, kept in the Library
/// with no conversation message. Any person in the company may add one, as they do an image to a
/// document or a file to a message (S65); it counts against their own kept-file quota and opens
/// only for them unless a document shows it or a message they send carries it.
pub(super) async fn upload_attachments(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    mut multipart: Multipart,
) -> Response<Body> {
    if !principal.permits_company(&company) {
        return api_error(
            StatusCode::FORBIDDEN,
            "company_out_of_scope",
            "this session is not scoped to that company",
        );
    }
    let mut files: Vec<(String, String, Vec<u8>)> = Vec::new();
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "attachment",
                    format!("the upload could not be read: {error}"),
                )
            }
        };
        if field.name() != Some("file") {
            continue;
        }
        if files.len() >= MAX_ATTACHMENTS {
            return api_error(
                StatusCode::BAD_REQUEST,
                "attachment",
                format!("Add at most {MAX_ATTACHMENTS} files at a time."),
            );
        }
        let name = safe_attachment_name(field.file_name().unwrap_or("file"));
        let media_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    "attachment",
                    format!("{name} could not be read: {error}"),
                )
            }
        };
        if bytes.len() > MAX_ATTACHMENT_BYTES {
            return api_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "attachment",
                format!(
                    "{name} is larger than {} MB, the most one file can be.",
                    MAX_ATTACHMENT_BYTES / (1024 * 1024)
                ),
            );
        }
        files.push((name, media_type, bytes.to_vec()));
    }
    if files.is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "attachment",
            "Choose at least one file to add.",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let sender = principal.actor_id().to_string();
    match org.active_actor(&sender).await {
        Ok(Some(row)) if row.actor_class == "human" => {}
        Ok(_) => {
            return api_error(
                StatusCode::FORBIDDEN,
                "request_principal",
                "the verified request principal is not an active human Actor",
            )
        }
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    }
    // Kept files live on the company computer's persistent disk. Adding one is real demand, so a
    // computer that slept while idle wakes for it; one the owner stopped stays stopped.
    if !restless_engine::runtime_sleep::wake_for(
        &state.daemon,
        &company,
        "someone added files",
    )
    .await
    {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "runtime",
            "The company computer is stopped, so files cannot be added. Start it from Company → Computer.",
        );
    }

    // Each upload is its own command: a retried upload adds a second copy, which the owner can
    // delete, rather than reusing a staging identity.
    let command_id = format!("library-upload:{}", Uuid::new_v4());
    let digest = {
        let files = files
            .iter()
            .map(|(name, media_type, bytes)| {
                serde_json::json!({
                    "name": name,
                    "media_type": media_type,
                    "bytes_sha256": format!("{:x}", Sha256::digest(bytes)),
                    "size_bytes": bytes.len(),
                })
            })
            .collect::<Vec<_>>();
        let command = serde_json::json!({
            "domain": "restless.owner-library-upload.v1",
            "company": company,
            "sender_actor": sender,
            "command_id": command_id,
            "files": files,
        });
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&command).expect("upload command serializes"))
        )
    };
    let prepared = files
        .into_iter()
        .map(|(name, media_type, bytes)| {
            let upload_id = Uuid::new_v4();
            PreparedOwnerAttachment {
                attachment: OwnerAttachment {
                    upload_id,
                    name,
                    media_type,
                    size_bytes: bytes.len(),
                    path: canonical_attachment_path(upload_id),
                },
                bytes,
            }
        })
        .collect::<Vec<_>>();
    if let Err(error) = collect_owner_attachments(&org, &company).await {
        tracing::warn!(%error, %company, "library upload staging cleanup failed");
        return files_not_stored();
    }
    let registrations = prepared
        .iter()
        .map(|item| restless_orgintel::OwnerAttachmentRegistration {
            attachment_id: item.attachment.upload_id,
            canonical_name: item.attachment.name.clone(),
            canonical_media_type: item.attachment.media_type.clone(),
            size_bytes: i64::try_from(item.attachment.size_bytes)
                .expect("attachment size is bounded"),
            content_sha256: format!("{:x}", Sha256::digest(&item.bytes)),
        })
        .collect::<Vec<_>>();
    if let Err(error) = org
        .register_owner_attachments(
            &sender,
            &sender,
            &command_id,
            &digest,
            &registrations,
            MAX_STAGED_ATTACHMENT_FILES as i64,
            MAX_STAGED_ATTACHMENT_BYTES as i64,
            MAX_RETAINED_ATTACHMENT_FILES as i64,
            MAX_RETAINED_ATTACHMENT_BYTES as i64,
            MAX_PRINCIPAL_RETAINED_ATTACHMENT_FILES as i64,
            MAX_PRINCIPAL_RETAINED_ATTACHMENT_BYTES as i64,
        )
        .await
    {
        return registration_refusal(error);
    }
    let staged = prepared
        .iter()
        .map(|item| item.attachment.clone())
        .collect::<Vec<_>>();
    for item in &prepared {
        if let Err(error) =
            runtime::store_owner_attachment(&company, item.attachment.upload_id, &item.bytes).await
        {
            tracing::warn!(%error, %company, "library upload could not store a file");
            rollback_attachment_attempt(&org, &company, &staged).await;
            return files_not_stored();
        }
        // Exec reads it by a name that keeps its extension, as it does a message's attachment.
        if let Err(error) =
            runtime::link_owner_attachment_name(&company, item.attachment.upload_id, &item.attachment.name).await
        {
            tracing::warn!(%error, %company, "library upload keeps only its canonical path");
        }
    }
    let ids = staged
        .iter()
        .map(|attachment| attachment.upload_id)
        .collect::<Vec<_>>();
    if let Err(error) = org.keep_owner_attachments(&sender, &command_id, &ids).await {
        tracing::warn!(%error, %company, "library upload could not keep its files");
        rollback_attachment_attempt(&org, &company, &staged).await;
        return files_not_stored();
    }
    Json(serde_json::json!({
        "attachments": staged
            .into_iter()
            .map(|attachment| UploadedAttachment {
                id: attachment.upload_id,
                name: attachment.name,
                media_type: attachment.media_type,
                size_bytes: attachment.size_bytes,
            })
            .collect::<Vec<_>>(),
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LibraryArchiveRequest {
    artifact_id: Uuid,
    archived: bool,
}

/// POST /companies/{company}/library/archive: take a Work file out of the Library, or put back
/// one the owner took out.
pub(super) async fn archive_work_file(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<LibraryArchiveRequest>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "membership_role",
            "only the company membership owner may archive Library files",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let result = if input.archived {
        org.retire_work_artifact(
            input.artifact_id,
            principal.actor_id(),
            "The owner archived this file from the Library.",
        )
        .await
    } else {
        org.restore_owner_retired_artifact(input.artifact_id, principal.actor_id())
            .await
    };
    match result {
        Ok(changed) => Json(serde_json::json!({
            "artifact_id": input.artifact_id,
            "archived": input.archived,
            "changed": changed,
        }))
        .into_response(),
        Err(restless_orgintel::OrgIntelError::InvalidWork(message)) => {
            api_error(StatusCode::CONFLICT, "library", message)
        }
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{error:#}"),
        ),
    }
}

/// GET /companies/{company}/library/archived: the Work files the owner archived.
pub(super) async fn archived_work_files(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "membership_role",
            "only the company membership owner may list archived Library files",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    match org.list_owner_retired_artifacts().await {
        Ok(artifacts) => Json(serde_json::json!({ "artifacts": artifacts })).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{error:#}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LibraryRenameRequest {
    /// A Work file, renamed everywhere it shows.
    #[serde(default)]
    artifact_id: Option<Uuid>,
    /// A kept file: `upload:<id>`; its own name returns with a null or empty label.
    #[serde(default)]
    key: Option<String>,
    label: Option<String>,
}

/// POST /companies/{company}/library/rename: the name a Work file or kept file shows.
pub(super) async fn rename_library_file(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<LibraryRenameRequest>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "membership_role",
            "only the company membership owner may rename Library files",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    let result = match (input.artifact_id, input.key.as_deref()) {
        (Some(artifact_id), None) => org
            .rename_work_artifact(artifact_id, input.label.as_deref().unwrap_or(""))
            .await
            .map(|changed| changed.then_some(())),
        (None, Some(key)) => org
            .set_library_label(principal.actor_id(), key, input.label.as_deref())
            .await
            .map(Some),
        _ => {
            return api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "library",
                "name one file: an artifact_id or a key",
            )
        }
    };
    match result {
        Ok(Some(())) => Json(serde_json::json!({ "renamed": true })).into_response(),
        Ok(None) => api_error(
            StatusCode::NOT_FOUND,
            "library",
            "this file is not one the company's Work recorded",
        ),
        Err(restless_orgintel::OrgIntelError::InvalidContext(message)) => {
            api_error(StatusCode::UNPROCESSABLE_ENTITY, "library", message)
        }
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{error:#}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LibraryRemoveRequest {
    artifact_id: Uuid,
}

/// POST /companies/{company}/library/remove: take an archived Work file out of the Library for
/// good. The file stays on the company computer, and Work's record of it stays.
pub(super) async fn remove_archived_work_file(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<LibraryRemoveRequest>,
) -> Response<Body> {
    if principal.membership_role() != "owner" {
        return api_error(
            StatusCode::FORBIDDEN,
            "membership_role",
            "only the company membership owner may remove Library files",
        );
    }
    let org = match state.daemon.orgintel.get(&company).await {
        Ok(org) => org,
        Err(error) => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "orgintel",
                format!("{error:#}"),
            )
        }
    };
    match org
        .remove_archived_work_artifact(input.artifact_id, principal.actor_id())
        .await
    {
        Ok(true) => Json(serde_json::json!({ "removed": true })).into_response(),
        Ok(false) => api_error(
            StatusCode::CONFLICT,
            "library",
            "only a file you archived can be removed from the Library",
        ),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "orgintel",
            format!("{error:#}"),
        ),
    }
}
